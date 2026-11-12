//! Colocar ventanas en Windows (Fase 6.5, ADR 0014): el `WindowManager` de verdad.
//!
//! - **Qué ventanas hay**: las normales y visibles. Fuera las ocultas, las «encubiertas»
//!   (apps de la Tienda en otro escritorio virtual), las de herramientas, las que tienen
//!   dueño (diálogos) y las de Sift.
//! - **Qué pantallas hay**: su área útil (sin la barra de tareas) y su nombre de modelo
//!   si Windows lo sabe.
//! - **Colocar**: se restaura si estaba maximizada o minimizada, se mueve compensando el
//!   borde invisible de Windows 10 y se repite una vez, por si al cambiar de pantalla
//!   (con otra escala) la app se redimensionó sola. «Toda la pantalla» se maximiza en esa
//!   pantalla.
//!
//! El proceso de Sift tiene en cuenta la escala de cada pantalla (per-monitor DPI v2): todas
//! las coordenadas son píxeles reales del escritorio.

use std::path::Path;

use sift_spaces::Action;
use sift_spaces::placement::{MonitorInfo, WindowInfo, WindowManager, WindowTarget, Zone};

pub struct Win32Windows;

impl WindowManager for Win32Windows {
    fn windows(&self) -> Vec<WindowInfo> {
        platform::windows()
    }

    fn monitors(&self) -> Vec<MonitorInfo> {
        platform::monitors()
    }

    fn mouse(&self) -> Option<(i32, i32)> {
        platform::mouse()
    }

    fn target(&self, action: &Action) -> WindowTarget {
        target_for(action, platform::default_exe)
    }

    fn place(&self, window: isize, zone: Zone, monitor: &MonitorInfo) -> Result<(), String> {
        platform::place(window, zone, monitor)
    }
}

/// Qué ventana buscar tras abrir una acción. `default_exe` dice qué programa abre un tipo
/// de archivo o protocolo (`.pdf`, `http`). Separado para probarlo sin Windows.
pub fn target_for(action: &Action, default_exe: impl Fn(&str) -> Option<String>) -> WindowTarget {
    let stem = |p: &Path| {
        p.file_stem()
            .or(p.file_name())
            .map(|s| s.to_string_lossy().into_owned())
    };
    match action {
        // El Explorador; el título es el nombre de la carpeta.
        Action::OpenFolder { path, .. } => WindowTarget {
            exe: Some("explorer.exe".into()),
            title_hint: path.file_name().map(|n| n.to_string_lossy().into_owned()),
        },
        // El programa que abre ese tipo; casi todos ponen el nombre del archivo en el título.
        Action::OpenFile { path, .. } => WindowTarget {
            exe: path
                .extension()
                .and_then(|e| default_exe(&format!(".{}", e.to_string_lossy()))),
            title_hint: stem(path),
        },
        // El navegador predeterminado.
        Action::OpenUrl { .. } => WindowTarget {
            exe: default_exe("http"),
            title_hint: None,
        },
        // Una app: si su id es la ruta de un .exe, ese; si es de la Tienda (un AUMID), no
        // se sabe el programa y se busca por su nombre en el título.
        Action::OpenApp { app_id, name, .. } => WindowTarget {
            exe: app_id
                .rsplit(['\\', '/'])
                .next()
                .filter(|last| last.to_ascii_lowercase().ends_with(".exe"))
                .map(str::to_ascii_lowercase),
            title_hint: Some(name.clone()),
        },
        _ => WindowTarget::default(),
    }
}

#[cfg(windows)]
#[allow(unsafe_code)] // Win32: cada bloque explica por qué es correcto.
mod platform {
    use std::path::Path;

    use sift_spaces::placement::{
        MonitorInfo, Rect, WindowInfo, Zone, with_invisible_borders, zone_rect,
    };
    use windows::Win32::Foundation::{CloseHandle, HWND, LPARAM, POINT, RECT};
    use windows::Win32::Graphics::Dwm::{
        DWMWA_CLOAKED, DWMWA_EXTENDED_FRAME_BOUNDS, DwmGetWindowAttribute,
    };
    use windows::Win32::Graphics::Gdi::{
        DISPLAY_DEVICEW, EnumDisplayDevicesW, EnumDisplayMonitors, GetMonitorInfoW, HDC, HMONITOR,
        MONITORINFO, MONITORINFOEXW,
    };
    use windows::Win32::System::Threading::{
        GetCurrentProcessId, OpenProcess, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION,
        QueryFullProcessImageNameW,
    };
    use windows::Win32::UI::Shell::{
        ASSOCF_IS_PROTOCOL, ASSOCF_NONE, ASSOCSTR_EXECUTABLE, AssocQueryStringW,
    };
    use windows::Win32::UI::WindowsAndMessaging::{
        EnumWindows, GW_OWNER, GWL_EXSTYLE, GetCursorPos, GetWindow, GetWindowLongPtrW,
        GetWindowRect, GetWindowTextLengthW, GetWindowTextW, GetWindowThreadProcessId, IsIconic,
        IsWindowVisible, IsZoomed, SW_MAXIMIZE, SW_RESTORE, SWP_NOACTIVATE, SWP_NOZORDER,
        SetForegroundWindow, SetWindowPos, ShowWindow, WS_EX_TOOLWINDOW,
    };
    use windows::core::{BOOL, HSTRING, PCWSTR, PWSTR};

    use Rect as R;

    const MONITORINFOF_PRIMARY: u32 = 1;

    fn to_rect(r: RECT) -> Rect {
        R {
            left: r.left,
            top: r.top,
            right: r.right,
            bottom: r.bottom,
        }
    }

    /// El .exe de un proceso, solo el nombre y en minúsculas.
    fn exe_of(pid: u32) -> Option<String> {
        // SAFETY: abrir con el permiso mínimo de consulta; el handle se cierra siempre y el
        // búfer vive durante la llamada, con su tamaño en `len`.
        unsafe {
            let process = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok()?;
            let mut buf = [0u16; 1024];
            let mut len = buf.len() as u32;
            let ok = QueryFullProcessImageNameW(
                process,
                PROCESS_NAME_WIN32,
                PWSTR(buf.as_mut_ptr()),
                &mut len,
            );
            let _ = CloseHandle(process);
            ok.ok()?;
            let path = String::from_utf16_lossy(&buf[..len as usize]);
            Path::new(&path)
                .file_name()
                .map(|n| n.to_string_lossy().to_ascii_lowercase())
        }
    }

    unsafe extern "system" fn collect_window(hwnd: HWND, lparam: LPARAM) -> BOOL {
        // SAFETY: `lparam` es el `&mut Vec<HWND>` que pasa `windows()` y vive mientras dura
        // `EnumWindows`, que llama a esto en el mismo hilo.
        let list = unsafe { &mut *(lparam.0 as *mut Vec<HWND>) };
        list.push(hwnd);
        BOOL(1)
    }

    pub fn windows() -> Vec<WindowInfo> {
        let mut handles: Vec<HWND> = Vec::new();
        // SAFETY: ver `collect_window`.
        unsafe {
            let _ = EnumWindows(
                Some(collect_window),
                LPARAM(&mut handles as *mut Vec<HWND> as isize),
            );
        }
        // SAFETY: funciona en cualquier momento.
        let own = unsafe { GetCurrentProcessId() };
        handles
            .into_iter()
            .filter_map(|hwnd| {
                // SAFETY: consultas de solo lectura sobre un HWND recién enumerado; si la
                // ventana ya no existe, devuelven 0/falso y se descarta.
                unsafe {
                    if !IsWindowVisible(hwnd).as_bool() {
                        return None;
                    }
                    if GetWindow(hwnd, GW_OWNER).is_ok_and(|o| !o.is_invalid()) {
                        return None;
                    }
                    let ex = GetWindowLongPtrW(hwnd, GWL_EXSTYLE) as u32;
                    if ex & WS_EX_TOOLWINDOW.0 != 0 {
                        return None;
                    }
                    let mut cloaked = 0u32;
                    if DwmGetWindowAttribute(
                        hwnd,
                        DWMWA_CLOAKED,
                        (&mut cloaked as *mut u32).cast(),
                        4,
                    )
                    .is_ok()
                        && cloaked != 0
                    {
                        return None;
                    }
                    let len = GetWindowTextLengthW(hwnd);
                    if len == 0 {
                        return None;
                    }
                    let mut buf = vec![0u16; len as usize + 1];
                    let n = GetWindowTextW(hwnd, &mut buf);
                    let title = String::from_utf16_lossy(&buf[..n as usize]);
                    let mut pid = 0u32;
                    GetWindowThreadProcessId(hwnd, Some(&mut pid));
                    if pid == own {
                        return None;
                    }
                    Some(WindowInfo {
                        id: hwnd.0 as isize,
                        exe: exe_of(pid).unwrap_or_default(),
                        title,
                    })
                }
            })
            .collect()
    }

    unsafe extern "system" fn collect_monitor(
        monitor: HMONITOR,
        _: HDC,
        _: *mut RECT,
        lparam: LPARAM,
    ) -> BOOL {
        // SAFETY: `lparam` es el `&mut Vec<HMONITOR>` de `monitors()`; ver `collect_window`.
        let list = unsafe { &mut *(lparam.0 as *mut Vec<HMONITOR>) };
        list.push(monitor);
        BOOL(1)
    }

    pub fn monitors() -> Vec<MonitorInfo> {
        let mut handles: Vec<HMONITOR> = Vec::new();
        // SAFETY: ver `collect_monitor`.
        unsafe {
            let _ = EnumDisplayMonitors(
                None,
                None,
                Some(collect_monitor),
                LPARAM(&mut handles as *mut Vec<HMONITOR> as isize),
            );
        }
        handles
            .into_iter()
            .enumerate()
            .filter_map(|(i, monitor)| {
                // SAFETY: estructuras con su tamaño puesto en `cbSize`, como piden las API;
                // viven durante cada llamada.
                unsafe {
                    let mut info = MONITORINFOEXW::default();
                    info.monitorInfo.cbSize = std::mem::size_of::<MONITORINFOEXW>() as u32;
                    if !GetMonitorInfoW(monitor, &mut info as *mut _ as *mut MONITORINFO).as_bool()
                    {
                        return None;
                    }
                    let mut device = DISPLAY_DEVICEW {
                        cb: std::mem::size_of::<DISPLAY_DEVICEW>() as u32,
                        ..Default::default()
                    };
                    let model =
                        EnumDisplayDevicesW(PCWSTR(info.szDevice.as_ptr()), 0, &mut device, 0)
                            .as_bool()
                            .then(|| {
                                let end = device
                                    .DeviceString
                                    .iter()
                                    .position(|&c| c == 0)
                                    .unwrap_or(device.DeviceString.len());
                                String::from_utf16_lossy(&device.DeviceString[..end])
                            })
                            .filter(|n| !n.is_empty() && !n.to_lowercase().contains("generic"));
                    Some(MonitorInfo {
                        name: model.unwrap_or_else(|| format!("Pantalla {}", i + 1)),
                        bounds: to_rect(info.monitorInfo.rcMonitor),
                        work: to_rect(info.monitorInfo.rcWork),
                        primary: info.monitorInfo.dwFlags & MONITORINFOF_PRIMARY != 0,
                    })
                }
            })
            .collect()
    }

    pub fn mouse() -> Option<(i32, i32)> {
        let mut p = POINT::default();
        // SAFETY: `p` vive durante la llamada.
        unsafe { GetCursorPos(&mut p) }.ok()?;
        Some((p.x, p.y))
    }

    /// El .exe que abre un tipo (`.pdf`) o un protocolo (`http`), en minúsculas.
    pub fn default_exe(kind: &str) -> Option<String> {
        let flags = if kind.starts_with('.') {
            ASSOCF_NONE
        } else {
            ASSOCF_IS_PROTOCOL
        };
        let mut buf = [0u16; 1024];
        let mut len = buf.len() as u32;
        // SAFETY: cadenas UTF-16 terminadas en NUL que viven durante la llamada; `buf` con
        // su tamaño en `len`.
        unsafe {
            AssocQueryStringW(
                flags,
                ASSOCSTR_EXECUTABLE,
                &HSTRING::from(kind),
                PCWSTR::null(),
                Some(PWSTR(buf.as_mut_ptr())),
                &mut len,
            )
        }
        .ok()
        .ok()?;
        let end = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
        Path::new(&String::from_utf16_lossy(&buf[..end]))
            .file_name()
            .map(|n| n.to_string_lossy().to_ascii_lowercase())
    }

    fn rects(hwnd: HWND) -> Option<(Rect, Rect)> {
        let mut window = RECT::default();
        let mut visible = RECT::default();
        // SAFETY: estructuras que viven durante cada llamada, con su tamaño.
        unsafe {
            GetWindowRect(hwnd, &mut window).ok()?;
            DwmGetWindowAttribute(
                hwnd,
                DWMWA_EXTENDED_FRAME_BOUNDS,
                (&mut visible as *mut RECT).cast(),
                std::mem::size_of::<RECT>() as u32,
            )
            .ok()?;
        }
        Some((to_rect(window), to_rect(visible)))
    }

    pub fn place(window: isize, zone: Zone, monitor: &MonitorInfo) -> Result<(), String> {
        let hwnd = HWND(window as *mut _);
        let move_to = |r: Rect| {
            // SAFETY: mover una ventana existente; si ya no existe, falla sin más.
            unsafe {
                SetWindowPos(
                    hwnd,
                    None,
                    r.left,
                    r.top,
                    r.width(),
                    r.height(),
                    SWP_NOZORDER | SWP_NOACTIVATE,
                )
            }
            .map_err(|e| format!("Windows no dejó moverla ({e})"))
        };
        // SAFETY: cambiar el estado de una ventana existente.
        unsafe {
            if IsIconic(hwnd).as_bool() || IsZoomed(hwnd).as_bool() {
                let _ = ShowWindow(hwnd, SW_RESTORE);
            }
        }
        let target = zone_rect(zone, monitor.work);
        if zone == Zone::Maximized {
            // Primero a esa pantalla; luego maximizada (se maximiza donde está).
            move_to(target)?;
            // SAFETY: como arriba.
            unsafe {
                let _ = ShowWindow(hwnd, SW_MAXIMIZE);
            }
        } else {
            // Dos veces: al pasar a una pantalla con otra escala, la app se redimensiona
            // sola y la primera vez puede quedarse corta.
            for _ in 0..2 {
                let (window_rect, visible) =
                    rects(hwnd).ok_or("no se pudo leer el tamaño de la ventana")?;
                let wanted = with_invisible_borders(target, window_rect, visible);
                if wanted != window_rect {
                    move_to(wanted)?;
                }
            }
        }
        // SAFETY: traer al frente una ventana existente (Windows puede negarse; no pasa nada).
        unsafe {
            let _ = SetForegroundWindow(hwnd);
        }
        Ok(())
    }
}

#[cfg(not(windows))]
mod platform {
    use sift_spaces::placement::{MonitorInfo, WindowInfo, Zone};

    pub fn windows() -> Vec<WindowInfo> {
        Vec::new()
    }
    pub fn monitors() -> Vec<MonitorInfo> {
        Vec::new()
    }
    pub fn mouse() -> Option<(i32, i32)> {
        None
    }
    pub fn default_exe(_: &str) -> Option<String> {
        None
    }
    pub fn place(_: isize, _: Zone, _: &MonitorInfo) -> Result<(), String> {
        Err("solo en Windows".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn exe(kind: &str) -> Option<String> {
        match kind {
            ".pdf" => Some("chrome.exe".into()),
            "http" => Some("firefox.exe".into()),
            _ => None,
        }
    }

    #[test]
    fn what_window_each_action_opens() {
        let folder = target_for(
            &Action::OpenFolder {
                path: PathBuf::from(r"C:\Uni\Apuntes"),
                place: None,
            },
            exe,
        );
        assert_eq!(folder.exe.as_deref(), Some("explorer.exe"));
        assert_eq!(folder.title_hint.as_deref(), Some("Apuntes"));

        let pdf = target_for(
            &Action::OpenFile {
                path: PathBuf::from(r"C:\Uni\tema4.pdf"),
                place: None,
            },
            exe,
        );
        assert_eq!(
            (pdf.exe.as_deref(), pdf.title_hint.as_deref()),
            (Some("chrome.exe"), Some("tema4"))
        );

        let web = target_for(
            &Action::OpenUrl {
                url: "https://x.es".into(),
                place: None,
            },
            exe,
        );
        assert_eq!(web.exe.as_deref(), Some("firefox.exe"));
    }

    #[test]
    fn apps_by_exe_path_or_by_name_if_from_the_store() {
        let app = |id: &str| {
            target_for(
                &Action::OpenApp {
                    app_id: id.into(),
                    name: "Visual Studio Code".into(),
                    place: None,
                },
                exe,
            )
        };
        let desktop = app(r"{6D809377-6AF0-444B-8957-A3773F02200E}\Microsoft VS Code\Code.exe");
        assert_eq!(desktop.exe.as_deref(), Some("code.exe"));
        let store = app("Microsoft.WindowsCalculator_8wekyb3d8bbwe!App");
        assert_eq!(store.exe, None);
        assert_eq!(store.title_hint.as_deref(), Some("Visual Studio Code"));
    }

    /// Qué ve Sift en este PC (solo lectura; depende del equipo).
    /// `cargo test -p sift-desktop window_manager -- --ignored --nocapture`
    #[test]
    #[ignore = "depende de las pantallas y ventanas de este equipo"]
    fn shows_monitors_and_windows_of_this_pc() {
        for m in Win32Windows.monitors() {
            eprintln!("pantalla: {m:?}");
        }
        for w in Win32Windows.windows() {
            eprintln!("ventana: {} · {}", w.exe, w.title);
        }
        eprintln!(
            ".pdf → {:?} · http → {:?}",
            platform::default_exe(".pdf"),
            platform::default_exe("http")
        );
    }

    /// Prueba en pantalla: abre ventanas de verdad, las coloca y mide dónde quedan. Las
    /// cierra al acabar. No usa ratón ni teclado.
    /// `cargo test -p sift-desktop places_real_windows -- --ignored --nocapture --test-threads=1`
    #[cfg(windows)]
    #[test]
    #[ignore = "abre y mueve ventanas en la pantalla"]
    #[allow(unsafe_code)] // medir dónde quedó la ventana y cerrarla: ver los SAFETY.
    fn places_real_windows() {
        use sift_spaces::placement::{Rect, pick_window, zone_rect};
        use std::time::{Duration, Instant};

        let wm = Win32Windows;
        let monitors = wm.monitors();
        let primary = monitors.iter().find(|m| m.primary).unwrap().clone();
        let other = monitors.iter().find(|m| !m.primary).cloned();

        // Lo que se ve de la ventana (sin el borde invisible) tiene que ser la zona.
        let check = |id: isize, zone: Zone, monitor: &MonitorInfo| {
            let hwnd = windows::Win32::Foundation::HWND(id as *mut _);
            let mut seen = windows::Win32::Foundation::RECT::default();
            // SAFETY: lectura de una ventana existente, con su tamaño.
            unsafe {
                windows::Win32::Graphics::Dwm::DwmGetWindowAttribute(
                    hwnd,
                    windows::Win32::Graphics::Dwm::DWMWA_EXTENDED_FRAME_BOUNDS,
                    (&mut seen as *mut windows::Win32::Foundation::RECT).cast(),
                    std::mem::size_of::<windows::Win32::Foundation::RECT>() as u32,
                )
                .unwrap();
            }
            let want = zone_rect(zone, monitor.work);
            let got = Rect {
                left: seen.left,
                top: seen.top,
                right: seen.right,
                bottom: seen.bottom,
            };
            eprintln!(
                "{zone:?} en {}: quería {want:?}, quedó {got:?}",
                monitor.name
            );
            for (a, b) in [
                (want.left, got.left),
                (want.top, got.top),
                (want.right, got.right),
                (want.bottom, got.bottom),
            ] {
                assert!(
                    (a - b).abs() <= 2,
                    "{zone:?}: quería {want:?}, quedó {got:?}"
                );
            }
        };
        let wait_new = |before: &[isize], target: &WindowTarget| {
            let started = Instant::now();
            loop {
                if let Some(w) = pick_window(&wm.windows(), before, target) {
                    return w.id;
                }
                assert!(
                    started.elapsed() < Duration::from_secs(15),
                    "no apareció la ventana"
                );
                std::thread::sleep(Duration::from_millis(200));
            }
        };

        // 1. El Bloc de notas.
        let before: Vec<isize> = wm.windows().iter().map(|w| w.id).collect();
        let mut notepad = std::process::Command::new("notepad.exe").spawn().unwrap();
        let id = wait_new(
            &before,
            &WindowTarget {
                exe: Some("notepad.exe".into()),
                title_hint: None,
            },
        );
        wm.place(id, Zone::RightHalf, &primary).unwrap();
        std::thread::sleep(Duration::from_millis(300));
        check(id, Zone::RightHalf, &primary);
        if let Some(other) = &other {
            wm.place(id, Zone::LeftThird, other).unwrap();
            std::thread::sleep(Duration::from_millis(300));
            check(id, Zone::LeftThird, other);
        }
        let _ = notepad.kill();
        let _ = notepad.wait();

        // 2. Todo el camino de Sift: una carpeta en el Explorador, en la mitad derecha.
        let dir = tempfile::tempdir().unwrap();
        let folder = dir.path().join("Prueba colocar Sift");
        std::fs::create_dir(&folder).unwrap();
        let action = Action::OpenFolder {
            path: folder.clone(),
            place: Some(sift_spaces::placement::Placement {
                zone: Zone::RightHalf,
                monitor: sift_spaces::placement::MonitorChoice::Primary,
                reuse_window: false,
            }),
        };
        let out = sift_spaces::run_actions(
            std::slice::from_ref(&action),
            &crate::shell::WindowsOpener,
            &crate::waits::LocalWaiter::new(),
            Some(&wm),
            &sift_spaces::NoProgress,
        );
        eprintln!("resultado: {out:?}");
        assert!(out[0].error.is_none() && out[0].note.is_none(), "{out:?}");
        let all = wm.windows();
        let explorer = pick_window(&all, &[], &wm.target(&action)).unwrap();
        std::thread::sleep(Duration::from_millis(300));
        check(explorer.id, Zone::RightHalf, &primary);
        // Cerrar la ventana del Explorador que se abrió.
        // SAFETY: pedir que se cierre una ventana existente (como pulsar la X).
        unsafe {
            let _ = windows::Win32::UI::WindowsAndMessaging::PostMessageW(
                Some(windows::Win32::Foundation::HWND(explorer.id as *mut _)),
                windows::Win32::UI::WindowsAndMessaging::WM_CLOSE,
                windows::Win32::Foundation::WPARAM(0),
                windows::Win32::Foundation::LPARAM(0),
            );
        }
    }
}
