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
        target_for(action, platform::default_exe, platform::app_path)
    }

    fn place(&self, window: isize, zone: Zone, monitor: &MonitorInfo) -> Result<(), String> {
        platform::place(window, zone, monitor)
    }

    fn focus(&self, window: isize) -> Result<(), String> {
        platform::focus(window)
    }
}

/// La carpeta de instalación de un programa (en minúsculas y acabada en `\`), para reconocer
/// las ventanas de otros programas de la misma carpeta (Opera abre `launcher.exe` y sus
/// ventanas son de `opera.exe`). Nada para rutas relativas, la raíz de un disco o la
/// carpeta de Windows: ahí hay programas de todo tipo.
fn install_folder(exe_path: &str) -> Option<String> {
    let lower = exe_path.to_lowercase().replace('/', "\\");
    let bytes = lower.as_bytes();
    let absolute = bytes.len() > 3 && bytes[1] == b':' && bytes[2] == b'\\';
    if !absolute {
        return None;
    }
    let parent = &lower[..lower.rfind('\\')?];
    let drive = &lower[..2];
    if parent.len() <= 3 || parent.starts_with(&format!("{drive}\\windows")) {
        return None;
    }
    Some(format!("{parent}\\"))
}

/// Qué ventana buscar tras abrir una acción. `default_exe` dice qué programa abre un tipo
/// de archivo o protocolo (`.pdf`, `http`); `app_path`, la ruta del programa de una app
/// registrada con un id (AUMID) en vez de con su ruta (Opera, VS Code). Separado para
/// probarlo sin Windows.
pub fn target_for(
    action: &Action,
    default_exe: impl Fn(&str) -> Option<String>,
    app_path: impl Fn(&str) -> Option<String>,
) -> WindowTarget {
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
            ..WindowTarget::default()
        },
        // El programa que abre ese tipo; casi todos ponen el nombre del archivo en el título.
        Action::OpenFile { path, .. } => WindowTarget {
            exe: path
                .extension()
                .and_then(|e| default_exe(&format!(".{}", e.to_string_lossy()))),
            title_hint: stem(path),
            ..WindowTarget::default()
        },
        // El navegador predeterminado.
        Action::OpenUrl { .. } => WindowTarget {
            exe: default_exe("http"),
            ..WindowTarget::default()
        },
        // Una app: si su id es la ruta de un .exe, ese y su carpeta. Si es un id de app
        // (AUMID), el id (las de la Tienda lo llevan en su proceso) y, si Windows sabe a qué
        // programa apunta su acceso directo, ese programa y su carpeta.
        Action::OpenApp { app_id, name, .. } => {
            let file = |path: &str| {
                path.rsplit(['\\', '/'])
                    .next()
                    .filter(|last| last.to_ascii_lowercase().ends_with(".exe"))
                    .map(str::to_ascii_lowercase)
            };
            if file(app_id).is_some() {
                WindowTarget {
                    exe: file(app_id),
                    folder: install_folder(app_id),
                    title_hint: Some(name.clone()),
                    ..WindowTarget::default()
                }
            } else {
                let path = app_path(app_id);
                WindowTarget {
                    exe: path.as_deref().and_then(file),
                    folder: path.as_deref().and_then(install_folder),
                    app_id: Some(app_id.clone()),
                    title_hint: Some(name.clone()),
                }
            }
        }
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
    use std::collections::HashMap;
    use windows::Win32::Foundation::{CloseHandle, HWND, LPARAM, POINT, RECT};
    use windows::Win32::Graphics::Dwm::{
        DWMWA_CLOAKED, DWMWA_EXTENDED_FRAME_BOUNDS, DwmGetWindowAttribute,
    };
    use windows::Win32::Graphics::Gdi::{
        DISPLAY_DEVICEW, EnumDisplayDevicesW, EnumDisplayMonitors, GetMonitorInfoW, HDC, HMONITOR,
        MONITORINFO, MONITORINFOEXW,
    };

    use windows::Win32::Storage::EnhancedStorage::{
        PKEY_AppUserModel_ID, PKEY_Link_TargetParsingPath,
    };
    use windows::Win32::Storage::Packaging::Appx::GetApplicationUserModelId;
    use windows::Win32::System::Com::StructuredStorage::PropVariantToStringAlloc;
    use windows::Win32::System::Com::{
        COINIT_MULTITHREADED, CoInitializeEx, CoTaskMemFree, CoUninitialize,
    };
    use windows::Win32::System::Diagnostics::ToolHelp::{
        CreateToolhelp32Snapshot, PROCESSENTRY32W, Process32FirstW, Process32NextW,
        TH32CS_SNAPPROCESS,
    };
    use windows::Win32::System::Threading::{
        GetCurrentProcessId, OpenProcess, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION,
        QueryFullProcessImageNameW,
    };
    use windows::Win32::UI::Shell::PropertiesSystem::{
        IPropertyStore, SHGetPropertyStoreForWindow,
    };
    use windows::Win32::UI::Shell::{
        ASSOCF_IS_PROTOCOL, ASSOCF_NONE, ASSOCSTR_EXECUTABLE, AssocQueryStringW,
    };
    use windows::Win32::UI::Shell::{IShellItem2, SHCreateItemFromParsingName};
    use windows::Win32::UI::WindowsAndMessaging::{
        EnumWindows, GW_OWNER, GWL_EXSTYLE, GWL_STYLE, GetCursorPos, GetWindow, GetWindowLongPtrW,
        GetWindowRect, GetWindowTextLengthW, GetWindowTextW, GetWindowThreadProcessId, IsIconic,
        IsWindowVisible, IsZoomed, SW_MAXIMIZE, SW_RESTORE, SWP_NOACTIVATE, SWP_NOZORDER,
        SetForegroundWindow, SetWindowPos, ShowWindow, WS_CAPTION, WS_EX_TOOLWINDOW,
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

    /// La ruta completa del programa de un proceso, en minúsculas.
    fn path_of(pid: u32) -> Option<String> {
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
            Some(String::from_utf16_lossy(&buf[..len as usize]).to_lowercase())
        }
    }

    fn file_name(path: &str) -> String {
        Path::new(path)
            .file_name()
            .map(|n| n.to_string_lossy().to_ascii_lowercase())
            .unwrap_or_default()
    }

    /// Todos los procesos: su padre y su programa (solo el nombre, en minúsculas).
    fn processes() -> HashMap<u32, (u32, String)> {
        let mut map = HashMap::new();
        // SAFETY: instantánea de solo lectura; la estructura lleva su tamaño en `dwSize`,
        // vive durante las llamadas y el handle se cierra al final.
        unsafe {
            let Ok(snap) = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) else {
                return map;
            };
            let mut entry = PROCESSENTRY32W {
                dwSize: std::mem::size_of::<PROCESSENTRY32W>() as u32,
                ..Default::default()
            };
            let mut ok = Process32FirstW(snap, &mut entry).is_ok();
            while ok {
                let end = entry
                    .szExeFile
                    .iter()
                    .position(|&c| c == 0)
                    .unwrap_or(entry.szExeFile.len());
                let exe = String::from_utf16_lossy(&entry.szExeFile[..end]).to_ascii_lowercase();
                map.insert(entry.th32ProcessID, (entry.th32ParentProcessID, exe));
                ok = Process32NextW(snap, &mut entry).is_ok();
            }
            let _ = CloseHandle(snap);
        }
        map
    }

    /// Los programas de los procesos padre de `pid`, del más cercano al más lejano (cinco
    /// como mucho; Windows reutiliza los números, así que se para si da la vuelta).
    fn ancestors(pid: u32, all: &HashMap<u32, (u32, String)>) -> Vec<String> {
        let mut out = Vec::new();
        let mut seen = vec![pid];
        let mut current = pid;
        while out.len() < 5 {
            let Some(&(parent, _)) = all.get(&current) else {
                break;
            };
            if parent == 0 || seen.contains(&parent) {
                break;
            }
            let Some((_, exe)) = all.get(&parent) else {
                break;
            };
            out.push(exe.clone());
            seen.push(parent);
            current = parent;
        }
        out
    }

    /// El id de app (AUMID) de una ventana: el que la propia ventana declara (muchas apps
    /// Electron) o, si no, el de su paquete (las apps de la Tienda, como Claude).
    fn app_id_of(hwnd: HWND, pid: u32) -> Option<String> {
        // SAFETY: interfaz COM obtenida de Windows para esta ventana (se libera sola); la
        // cadena que devuelve se libera con CoTaskMemFree. Si la ventana ya no existe,
        // falla sin más.
        let declared = unsafe {
            SHGetPropertyStoreForWindow::<IPropertyStore>(hwnd)
                .and_then(|store| store.GetValue(&PKEY_AppUserModel_ID))
                .and_then(|value| PropVariantToStringAlloc(&value))
                .ok()
                .and_then(|text| {
                    let s = text.to_string().ok();
                    CoTaskMemFree(Some(text.0 as *const _));
                    s
                })
        };
        if let Some(id) = declared.filter(|s| !s.is_empty()) {
            return Some(id);
        }
        // SAFETY: abrir con el permiso mínimo de consulta; el búfer vive durante la llamada
        // con su tamaño en `len`, y el handle se cierra siempre.
        unsafe {
            let process = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok()?;
            let mut buf = [0u16; 512];
            let mut len = buf.len() as u32;
            let result =
                GetApplicationUserModelId(process, &mut len, Some(PWSTR(buf.as_mut_ptr())));
            let _ = CloseHandle(process);
            if result.is_err() {
                return None;
            }
            let end = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
            Some(String::from_utf16_lossy(&buf[..end])).filter(|s| !s.is_empty())
        }
    }

    /// COM en este hilo mientras dura `f` (lo necesitan las propiedades del Shell).
    fn with_com<T>(f: impl FnOnce() -> T) -> T {
        // SAFETY: inicializar y cerrar COM en el mismo hilo, emparejados. Si el hilo ya
        // tenía COM con otro modelo, falla, se usa el que había y no se cierra.
        let init = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) };
        let out = f();
        if init.is_ok() {
            // SAFETY: emparejado con el CoInitializeEx de arriba, que salió bien.
            unsafe { CoUninitialize() };
        }
        out
    }

    /// La ruta del programa al que apunta una app registrada con un id (AUMID), si Windows
    /// la sabe: las de escritorio con acceso directo (Opera, VS Code) sí; las de la Tienda,
    /// no. En minúsculas.
    pub fn app_path(app_id: &str) -> Option<String> {
        with_com(|| {
            // SAFETY: interfaz COM del Shell (se libera sola); la cadena que devuelve se
            // libera con CoTaskMemFree.
            unsafe {
                let item: IShellItem2 = SHCreateItemFromParsingName(
                    &HSTRING::from(format!("shell:AppsFolder\\{app_id}")),
                    None,
                )
                .ok()?;
                let text = item.GetString(&PKEY_Link_TargetParsingPath).ok()?;
                let path = text.to_string().ok();
                CoTaskMemFree(Some(text.0 as *const _));
                path.filter(|p| !p.is_empty()).map(|p| p.to_lowercase())
            }
        })
    }

    unsafe extern "system" fn collect_window(hwnd: HWND, lparam: LPARAM) -> BOOL {
        // SAFETY: `lparam` es el `&mut Vec<HWND>` que pasa `windows()` y vive mientras dura
        // `EnumWindows`, que llama a esto en el mismo hilo.
        let list = unsafe { &mut *(lparam.0 as *mut Vec<HWND>) };
        list.push(hwnd);
        BOOL(1)
    }

    pub fn windows() -> Vec<WindowInfo> {
        with_com(windows_inner)
    }

    fn windows_inner() -> Vec<WindowInfo> {
        let all = processes();
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
                    let style = GetWindowLongPtrW(hwnd, GWL_STYLE) as u32;
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
                    let path = path_of(pid).unwrap_or_default();
                    let mut rect = RECT::default();
                    let _ = GetWindowRect(hwnd, &mut rect);
                    Some(WindowInfo {
                        id: hwnd.0 as isize,
                        exe: if path.is_empty() {
                            all.get(&pid).map(|(_, e)| e.clone()).unwrap_or_default()
                        } else {
                            file_name(&path)
                        },
                        title,
                        ancestors: ancestors(pid, &all),
                        app_id: app_id_of(hwnd, pid),
                        size: (rect.right - rect.left, rect.bottom - rect.top),
                        frameless: style & WS_CAPTION.0 != WS_CAPTION.0,
                        path,
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

    /// Trae al frente una ventana ya abierta, sin moverla («usar la que ya está abierta»).
    pub fn focus(window: isize) -> Result<(), String> {
        let hwnd = HWND(window as *mut _);
        // SAFETY: restaurar y traer al frente una ventana existente; si ya no existe, las
        // llamadas fallan sin más. Windows puede negarse a darle el foco: no es un error.
        unsafe {
            if IsIconic(hwnd).as_bool() {
                let _ = ShowWindow(hwnd, SW_RESTORE);
            }
            let _ = SetForegroundWindow(hwnd);
        }
        Ok(())
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
    pub fn app_path(_: &str) -> Option<String> {
        None
    }
    pub fn focus(_: isize) -> Result<(), String> {
        Err("solo en Windows".into())
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

    fn no_app_path(_: &str) -> Option<String> {
        None
    }

    /// Como Windows: los accesos directos de escritorio sí dicen su programa; los de la
    /// Tienda, no.
    fn app_path(app_id: &str) -> Option<String> {
        match app_id {
            "OperaSoftware.OperaGXWebBrowser.1759849920" => {
                Some(r"c:\users\ana\appdata\local\programs\opera gx\launcher.exe".into())
            }
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
            no_app_path,
        );
        assert_eq!(folder.exe.as_deref(), Some("explorer.exe"));
        assert_eq!(folder.title_hint.as_deref(), Some("Apuntes"));

        let pdf = target_for(
            &Action::OpenFile {
                path: PathBuf::from(r"C:\Uni\tema4.pdf"),
                place: None,
            },
            exe,
            no_app_path,
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
            no_app_path,
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
                app_path,
            )
        };
        let desktop = app(r"{6D809377-6AF0-444B-8957-A3773F02200E}\Microsoft VS Code\Code.exe");
        assert_eq!(desktop.exe.as_deref(), Some("code.exe"));
        // Con una ruta que empieza por una carpeta conocida no se sabe la carpeta real.
        assert_eq!(desktop.folder, None);
        let eclipse = app(r"C:\Users\Ana\eclipse\java-2025-09\eclipse\eclipse.exe");
        assert_eq!(
            eclipse.folder.as_deref(),
            Some(r"c:\users\ana\eclipse\java-2025-09\eclipse\")
        );
        // De la Tienda: su id, sin programa.
        let store = app("Claude_pzs8sxrjxfjjc!Claude");
        assert_eq!(store.exe, None);
        assert_eq!(store.app_id.as_deref(), Some("Claude_pzs8sxrjxfjjc!Claude"));
        assert_eq!(store.title_hint.as_deref(), Some("Visual Studio Code"));
        // Con id de app y acceso directo (Opera): el id, el programa y su carpeta.
        let opera = app("OperaSoftware.OperaGXWebBrowser.1759849920");
        assert_eq!(opera.exe.as_deref(), Some("launcher.exe"));
        assert_eq!(
            opera.folder.as_deref(),
            Some(r"c:\users\ana\appdata\local\programs\opera gx\")
        );
        assert!(opera.app_id.is_some());
    }

    #[test]
    fn the_windows_folder_is_never_an_install_folder() {
        assert_eq!(install_folder(r"C:\Windows\System32\notepad.exe"), None);
        assert_eq!(install_folder(r"C:\notepad.exe"), None);
        assert_eq!(install_folder("notepad.exe"), None);
        assert_eq!(
            install_folder(r"D:\Apps\Tool\tool.exe").as_deref(),
            Some(r"d:\apps\tool\")
        );
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
            eprintln!(
                "ventana: {} · {} · app={:?} · padres={:?} · {:?}",
                w.exe, w.title, w.app_id, w.ancestors, w.size
            );
        }
        for id in [
            "Claude_pzs8sxrjxfjjc!Claude",
            "OperaSoftware.OperaGXWebBrowser.1759849920",
            "Microsoft.VisualStudioCode",
        ] {
            eprintln!("programa de {id}: {:?}", platform::app_path(id));
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
                ..WindowTarget::default()
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
                zone: Some(Zone::RightHalf),
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

    /// Prueba a mano de que las acciones no se esperan (ADR 0014, cambio del 2026-11-19):
    /// Eclipse (con su workspace ya en uso: tras su pantalla de carga solo enseña su selector) y una ventana
    /// nueva de Opera GX. Opera se coloca sin esperar a Eclipse; ni la pantalla de carga ni
    /// el selector se colocan. Cierra solo las ventanas que abrió.
    /// `cargo test -p sift-desktop slow_apps_do_not_block -- --ignored --nocapture`
    #[cfg(windows)]
    #[test]
    #[ignore = "abre y mueve ventanas en la pantalla"]
    #[allow(unsafe_code)] // cerrar las ventanas abiertas: ver el SAFETY.
    fn slow_apps_do_not_block_the_rest() {
        use sift_spaces::placement::{MonitorChoice, Placement};
        use sift_spaces::{PlacementState, Progress};
        use std::time::Instant;

        struct Timeline(Instant);
        impl Progress for Timeline {
            fn step(&self, index: u32, label: &str, waited: Option<u32>) {
                eprintln!(
                    "{:>6} ms  paso {index}: {label} {waited:?}",
                    self.0.elapsed().as_millis()
                );
            }
            fn cancelled(&self) -> bool {
                false
            }
            fn placement(&self, index: u32, state: PlacementState) {
                eprintln!(
                    "{:>6} ms  paso {index}: {state:?}",
                    self.0.elapsed().as_millis()
                );
            }
        }

        let open = |app_id: &str, name: &str, zone: Zone| Action::OpenApp {
            app_id: app_id.into(),
            name: name.into(),
            place: Some(Placement {
                zone: Some(zone),
                monitor: MonitorChoice::Primary,
                reuse_window: false,
            }),
        };
        let eclipse =
            std::env::var("USERPROFILE").unwrap() + r"\eclipse\java-2025-09\eclipse\eclipse.exe";
        let actions = [
            open(&eclipse, "Eclipse", Zone::RightHalf),
            open(
                "OperaSoftware.OperaGXWebBrowser.1759849920",
                "Opera GX",
                Zone::LeftHalf,
            ),
        ];
        let wm = Win32Windows;
        let before: Vec<isize> = wm.windows().iter().map(|w| w.id).collect();
        let out = sift_spaces::run_actions(
            &actions,
            &crate::shell::WindowsOpener,
            &crate::waits::LocalWaiter::new(),
            Some(&wm),
            &Timeline(Instant::now()),
        );
        for o in &out {
            eprintln!(
                "resultado {}: {} · {:?} · {:?}",
                o.index, o.label, o.error, o.note
            );
        }
        // Cerrar solo lo que se abrió (también el selector de Eclipse).
        for w in wm.windows().iter().filter(|w| !before.contains(&w.id)) {
            let ours = ["opera.exe", "eclipse.exe", "javaw.exe"];
            if !ours.contains(&w.exe.as_str()) {
                continue;
            }
            eprintln!("cierro {} «{}»", w.exe, w.title);
            // SAFETY: pedir que se cierre una ventana existente (como pulsar la X).
            unsafe {
                let _ = windows::Win32::UI::WindowsAndMessaging::PostMessageW(
                    Some(windows::Win32::Foundation::HWND(w.id as *mut _)),
                    windows::Win32::UI::WindowsAndMessaging::WM_CLOSE,
                    windows::Win32::Foundation::WPARAM(0),
                    windows::Win32::Foundation::LPARAM(0),
                );
            }
        }
        assert!(out[1].note.is_none(), "{out:?}");
    }
}
