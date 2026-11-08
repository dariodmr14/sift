//! Atajo global de teclado.
//!
//! Estrategia (ver ADR 0002):
//! 1. `RegisterHotKey`: la vía oficial de Windows. El sistema detecta la combinación y nos
//!    avisa con `WM_HOTKEY`; además se "come" la pulsación, así que `Alt+Space` no abre el
//!    menú de ventana de la app que tengas delante.
//! 2. Si falla (otra app ya tiene ese atajo, o Windows lo reserva), se instala un hook de
//!    teclado de bajo nivel (`WH_KEYBOARD_LL`): vemos cada tecla antes que nadie y nos
//!    tragamos la combinación. Es lo que hace PowerToys Run.
//!
//! Las dos vías necesitan un hilo con bucle de mensajes, así que el atajo vive en un hilo
//! propio. Para pararlo se le manda `WM_QUIT` y se espera a que termine.

use std::sync::mpsc;
use std::thread::JoinHandle;

use serde::Serialize;
use sift_core::Hotkey;

/// Con qué mecanismo se está escuchando el atajo.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum Backend {
    RegisterHotKey,
    KeyboardHook,
}

/// Escucha un atajo mientras exista. Al destruirse (`Drop`) lo libera.
pub struct HotkeyListener {
    hotkey: Hotkey,
    backend: Backend,
    thread_id: u32,
    thread: Option<JoinHandle<()>>,
}

impl HotkeyListener {
    /// Empieza a escuchar `hotkey`. `on_press` se llama en el hilo del atajo cada vez que
    /// se pulsa: tiene que volver enseguida (si el hook tarda más de ~1 s, Windows lo quita).
    ///
    /// Con `force_hook` se salta `RegisterHotKey` (útil para probar la vía alternativa).
    pub fn start(
        hotkey: Hotkey,
        force_hook: bool,
        on_press: impl Fn() + Send + 'static,
    ) -> anyhow::Result<HotkeyListener> {
        let (ready_tx, ready_rx) = mpsc::channel();
        let thread = std::thread::Builder::new()
            .name("sift-hotkey".into())
            .spawn(move || platform::run(hotkey, force_hook, on_press, ready_tx))?;

        // El hilo nos dice si consiguió registrar el atajo antes de entrar en su bucle.
        match ready_rx.recv() {
            Ok(Ok((thread_id, backend))) => {
                tracing::info!(%hotkey, ?backend, thread_id, "atajo global activo");
                Ok(HotkeyListener {
                    hotkey,
                    backend,
                    thread_id,
                    thread: Some(thread),
                })
            }
            Ok(Err(message)) => {
                let _ = thread.join();
                anyhow::bail!("no se pudo activar el atajo {hotkey}: {message}")
            }
            Err(_) => anyhow::bail!("el hilo del atajo terminó de forma inesperada"),
        }
    }

    pub fn hotkey(&self) -> Hotkey {
        self.hotkey
    }

    pub fn backend(&self) -> Backend {
        self.backend
    }
}

impl Drop for HotkeyListener {
    fn drop(&mut self) {
        platform::stop(self.thread_id);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
        tracing::info!(hotkey = %self.hotkey, "atajo global liberado");
    }
}

type Ready = mpsc::Sender<Result<(u32, Backend), String>>;

#[cfg(windows)]
#[allow(unsafe_code)] // Llamadas a Win32: cada bloque `unsafe` explica por qué es correcto.
mod platform {
    use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

    use sift_core::{Hotkey, Modifiers};
    use windows_sys::Win32::Foundation::{GetLastError, LPARAM, LRESULT, WPARAM};
    use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
    use windows_sys::Win32::System::Threading::GetCurrentThreadId;
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
        GetAsyncKeyState, INPUT, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP, MOD_ALT, MOD_CONTROL,
        MOD_NOREPEAT, MOD_SHIFT, MOD_WIN, RegisterHotKey, SendInput, UnregisterHotKey, VK_CONTROL,
        VK_LWIN, VK_MENU, VK_RWIN, VK_SHIFT,
    };
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        CallNextHookEx, GetMessageW, HC_ACTION, KBDLLHOOKSTRUCT, MSG, PM_NOREMOVE, PeekMessageW,
        PostThreadMessageW, SetWindowsHookExW, UnhookWindowsHookEx, WH_KEYBOARD_LL, WM_APP,
        WM_HOTKEY, WM_KEYDOWN, WM_KEYUP, WM_QUIT, WM_SYSKEYDOWN, WM_SYSKEYUP, WM_USER,
    };

    use super::{Backend, Ready};

    /// Identificador del atajo para `RegisterHotKey` (único dentro de este hilo).
    const HOTKEY_ID: i32 = 1;
    /// Mensaje que el hook manda a su propio hilo cuando detecta la combinación.
    const WM_APP_HOTKEY: u32 = WM_APP + 1;
    /// Tecla virtual sin asignar que se inyecta para "romper" la secuencia (ver `send_dummy_key`).
    const VK_DUMMY: u16 = 0xE8;

    // Estado del hook. El callback del hook es una función `extern "system"` sin
    // contexto, así que lo que necesita va en estáticos atómicos (lectura sin bloqueos).
    // Solo hay un hook a la vez: el anterior se para antes de crear uno nuevo.
    static HOOK_VK: AtomicU32 = AtomicU32::new(0);
    static HOOK_MODS: AtomicU32 = AtomicU32::new(0);
    static HOOK_THREAD: AtomicU32 = AtomicU32::new(0);
    static HOOK_KEY_HELD: AtomicBool = AtomicBool::new(false);
    /// Modificadores que el hook ha visto pulsar y aún no soltar (en flags `MOD_*`).
    static HOOK_SEEN_MODS: AtomicU32 = AtomicU32::new(0);

    fn mod_flags(m: Modifiers) -> u32 {
        let mut flags = 0;
        if m.ctrl {
            flags |= MOD_CONTROL;
        }
        if m.alt {
            flags |= MOD_ALT;
        }
        if m.shift {
            flags |= MOD_SHIFT;
        }
        if m.win {
            flags |= MOD_WIN;
        }
        flags
    }

    pub fn run(hotkey: Hotkey, force_hook: bool, on_press: impl Fn(), ready: Ready) {
        let vk = u32::from(hotkey.key.vk_code());
        let mods = mod_flags(hotkey.modifiers);

        // SAFETY: todas las llamadas son a funciones de Win32 con punteros a variables
        // locales que viven durante la llamada; los handles se usan solo en este hilo.
        unsafe {
            let thread_id = GetCurrentThreadId();
            // Un hilo no tiene cola de mensajes hasta que llama a una función de mensajes.
            // La creamos ya, para que `PostThreadMessageW` (parar o el hook) no se pierda.
            let mut msg: MSG = std::mem::zeroed();
            PeekMessageW(
                &mut msg,
                std::ptr::null_mut(),
                WM_USER,
                WM_USER,
                PM_NOREMOVE,
            );

            let mut hook = std::ptr::null_mut();
            let backend = if !force_hook
                && RegisterHotKey(std::ptr::null_mut(), HOTKEY_ID, mods | MOD_NOREPEAT, vk) != 0
            {
                Backend::RegisterHotKey
            } else {
                if !force_hook {
                    tracing::warn!(
                        %hotkey,
                        code = GetLastError(),
                        "RegisterHotKey falló (¿atajo ocupado?); se usa un hook de teclado"
                    );
                }
                HOOK_VK.store(vk, Ordering::SeqCst);
                HOOK_MODS.store(mods, Ordering::SeqCst);
                HOOK_THREAD.store(thread_id, Ordering::SeqCst);
                HOOK_KEY_HELD.store(false, Ordering::SeqCst);
                HOOK_SEEN_MODS.store(0, Ordering::SeqCst);
                hook = SetWindowsHookExW(
                    WH_KEYBOARD_LL,
                    Some(keyboard_hook),
                    GetModuleHandleW(std::ptr::null()),
                    0,
                );
                if hook.is_null() {
                    let code = GetLastError();
                    let _ = ready.send(Err(format!("SetWindowsHookExW falló (código {code})")));
                    return;
                }
                Backend::KeyboardHook
            };
            let _ = ready.send(Ok((thread_id, backend)));

            // Bucle de mensajes: GetMessageW devuelve 0 con WM_QUIT y -1 si hay error.
            loop {
                let r = GetMessageW(&mut msg, std::ptr::null_mut(), 0, 0);
                if r == 0 || r == -1 {
                    break;
                }
                let ours = match msg.message {
                    WM_HOTKEY => msg.wParam == HOTKEY_ID as usize,
                    WM_APP_HOTKEY => true,
                    _ => false,
                };
                if ours {
                    on_press();
                }
            }

            match backend {
                Backend::RegisterHotKey => {
                    UnregisterHotKey(std::ptr::null_mut(), HOTKEY_ID);
                }
                Backend::KeyboardHook => {
                    UnhookWindowsHookEx(hook);
                    HOOK_THREAD.store(0, Ordering::SeqCst);
                }
            }
        }
    }

    pub fn stop(thread_id: u32) {
        // SAFETY: mandar un mensaje a un hilo por su id no toca memoria nuestra. Si el hilo
        // ya terminó, la llamada simplemente falla.
        unsafe {
            PostThreadMessageW(thread_id, WM_QUIT, 0, 0);
        }
    }

    fn is_down(vk: u16) -> bool {
        // SAFETY: GetAsyncKeyState solo lee el estado del teclado.
        // El bit alto (0x8000) indica que la tecla está pulsada ahora mismo.
        (unsafe { GetAsyncKeyState(i32::from(vk)) } as u16) & 0x8000 != 0
    }

    /// Si `vk` es un modificador, devuelve su flag `MOD_*`. El hook recibe las variantes
    /// izquierda/derecha (`VK_LSHIFT`…), y las genéricas si alguien inyecta `VK_SHIFT`.
    fn modifier_flag(vk: u32) -> Option<u32> {
        match vk {
            0x10 | 0xA0 | 0xA1 => Some(MOD_SHIFT), // VK_SHIFT, VK_LSHIFT, VK_RSHIFT
            0x11 | 0xA2 | 0xA3 => Some(MOD_CONTROL), // VK_CONTROL, VK_LCONTROL, VK_RCONTROL
            0x12 | 0xA4 | 0xA5 => Some(MOD_ALT),   // VK_MENU, VK_LMENU, VK_RMENU
            0x5B | 0x5C => Some(MOD_WIN),          // VK_LWIN, VK_RWIN
            _ => None,
        }
    }

    /// Modificadores pulsados ahora. Se combinan dos fuentes, por robustez:
    /// - Lo que el propio hook ha visto pasar (no depende del estado global del teclado,
    ///   que dentro de un hook refleja los eventos ya procesados, no el actual).
    /// - `GetAsyncKeyState`: cubre modificadores pulsados antes de instalar el hook.
    fn current_mods() -> u32 {
        let mut m = HOOK_SEEN_MODS.load(Ordering::Relaxed);
        if is_down(VK_CONTROL) {
            m |= MOD_CONTROL;
        }
        if is_down(VK_MENU) {
            m |= MOD_ALT;
        }
        if is_down(VK_SHIFT) {
            m |= MOD_SHIFT;
        }
        if is_down(VK_LWIN) || is_down(VK_RWIN) {
            m |= MOD_WIN;
        }
        m
    }

    /// Inyecta una pulsación de una tecla que no existe. Sirve para dos cosas:
    /// - Que al soltar Alt (o Win) Windows no active la barra de menús de la app de
    ///   delante (o el menú Inicio): ya no es "Alt pulsado y soltado solo".
    /// - Que nuestro proceso haya generado la última entrada, condición que Windows exige
    ///   para dejarnos poner la ventana en primer plano con `SetForegroundWindow`.
    fn send_dummy_key() {
        let key = |flags| INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: windows_sys::Win32::UI::Input::KeyboardAndMouse::INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: VK_DUMMY,
                    wScan: 0,
                    dwFlags: flags,
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        };
        let inputs = [key(0), key(KEYEVENTF_KEYUP)];
        // SAFETY: `inputs` es un array válido de 2 INPUT y le pasamos su tamaño real.
        unsafe {
            SendInput(
                inputs.len() as u32,
                inputs.as_ptr(),
                std::mem::size_of::<INPUT>() as i32,
            );
        }
    }

    /// Callback del hook: Windows lo llama con cada tecla, de cualquier app.
    unsafe extern "system" fn keyboard_hook(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
        if code == HC_ACTION as i32 {
            // SAFETY: con WH_KEYBOARD_LL y HC_ACTION, `lparam` apunta a un KBDLLHOOKSTRUCT
            // válido durante esta llamada (contrato documentado de Win32).
            let kb = unsafe { &*(lparam as *const KBDLLHOOKSTRUCT) };
            // No se filtran las teclas inyectadas (AutoHotkey, escritorio remoto…): nuestra
            // tecla falsa (VK_DUMMY) nunca coincide con la del atajo, así que no hay bucle.
            let msg = wparam as u32;
            let down = msg == WM_KEYDOWN || msg == WM_SYSKEYDOWN;
            let up = msg == WM_KEYUP || msg == WM_SYSKEYUP;
            if let Some(flag) = modifier_flag(kb.vkCode) {
                if down {
                    HOOK_SEEN_MODS.fetch_or(flag, Ordering::Relaxed);
                } else if up {
                    HOOK_SEEN_MODS.fetch_and(!flag, Ordering::Relaxed);
                }
            } else if kb.vkCode == HOOK_VK.load(Ordering::Relaxed) {
                unsafe {
                    PostThreadMessageW(
                        HOOK_THREAD.load(Ordering::Relaxed),
                        WM_APP + 2,
                        msg as usize,
                        current_mods() as isize,
                    );
                }
                if down && current_mods() == HOOK_MODS.load(Ordering::Relaxed) {
                    // Si se mantiene pulsada, Windows repite keydown: solo cuenta la primera.
                    if !HOOK_KEY_HELD.swap(true, Ordering::Relaxed) {
                        send_dummy_key();
                        // SAFETY: solo envía un mensaje a la cola de nuestro hilo.
                        unsafe {
                            PostThreadMessageW(
                                HOOK_THREAD.load(Ordering::Relaxed),
                                WM_APP_HOTKEY,
                                0,
                                0,
                            );
                        }
                    }
                    return 1; // no pasa a nadie más: nos la tragamos
                }
                if up && HOOK_KEY_HELD.swap(false, Ordering::Relaxed) {
                    return 1; // también el keyup de la tecla que nos tragamos
                }
            }
        }
        // SAFETY: pasar el evento al siguiente hook con los mismos argumentos que recibimos.
        unsafe { CallNextHookEx(std::ptr::null_mut(), code, wparam, lparam) }
    }
}

#[cfg(not(windows))]
mod platform {
    use super::Ready;
    use sift_core::Hotkey;

    pub fn run(_: Hotkey, _: bool, _: impl Fn(), ready: Ready) {
        let _ = ready.send(Err(
            "el atajo global solo está implementado en Windows".into()
        ));
    }

    pub fn stop(_: u32) {}
}
