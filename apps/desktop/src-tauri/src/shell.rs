//! Lo que la app necesita del Shell de Windows:
//! - Listar las aplicaciones instaladas (las mismas que el menú Inicio, incluidas las de
//!   la Store) para el selector de apps.
//! - El icono real de Windows de una app, archivo o carpeta, como PNG en una data URL.
//! - El `Opener` real con el que `sift-spaces` lanza las acciones.
//!
//! Todo pasa por la carpeta virtual `shell:AppsFolder` y por `IShellItem` (COM).

use std::collections::HashMap;
use std::path::Path;
use std::sync::Mutex;

use serde::Serialize;
use sift_spaces::{CommandSpec, Opener};

/// Una app instalada, para el selector.
#[derive(Debug, Clone, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct InstalledApp {
    /// Identificador en `shell:AppsFolder` (un AUMID o una ruta).
    pub id: String,
    pub name: String,
}

/// Caché de iconos ya extraídos (clave: ruta o `app:<id>`). Extraer un icono cuesta unos
/// milisegundos; con la caché, abrir la barra no repite el trabajo.
#[derive(Default)]
pub struct IconCache(Mutex<HashMap<String, Option<String>>>);

impl IconCache {
    pub fn get_or_load(&self, key: &str) -> Option<String> {
        if let Some(hit) = self.0.lock().unwrap_or_else(|p| p.into_inner()).get(key) {
            return hit.clone();
        }
        let loaded = platform::icon_data_url(&parsing_name(key));
        self.0
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .insert(key.to_owned(), loaded.clone());
        loaded
    }
}

/// Clave de caché para el icono de una app.
pub fn app_icon_key(app_id: &str) -> String {
    format!("app:{app_id}")
}

fn parsing_name(key: &str) -> String {
    match key.strip_prefix("app:") {
        Some(id) => format!("shell:AppsFolder\\{id}"),
        None => key.to_owned(),
    }
}

pub fn list_apps() -> Result<Vec<InstalledApp>, String> {
    let mut apps = platform::list_apps()?;
    apps.retain(|a| !a.name.trim().is_empty());
    apps.sort_by_key(|a| a.name.to_lowercase());
    apps.dedup_by(|a, b| a.name == b.name && a.id == b.id);
    Ok(apps)
}

/// El `Opener` de verdad.
pub struct WindowsOpener;

impl Opener for WindowsOpener {
    fn open_path(&self, path: &Path) -> Result<(), String> {
        platform::shell_open(&path.display().to_string())
    }

    fn open_url(&self, url: &str) -> Result<(), String> {
        platform::shell_open(url)
    }

    fn open_app(&self, app_id: &str) -> Result<(), String> {
        platform::shell_open(&format!("shell:AppsFolder\\{app_id}"))
    }

    fn spawn(&self, command: &CommandSpec) -> Result<(), String> {
        let mut cmd = std::process::Command::new(&command.program);
        // Cada argumento por separado: Rust los entrecomilla bien al pasarlos a Windows,
        // y para .cmd/.bat se niega a lanzar si un argumento no se puede escapar seguro.
        cmd.args(&command.args);
        if let Some(cwd) = &command.cwd {
            cmd.current_dir(cwd);
        }
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            // Consola propia: si es un programa de consola (docker, npm…), ves su salida
            // en su ventana; si es una app con ventana, se ignora.
            const CREATE_NEW_CONSOLE: u32 = 0x0000_0010;
            cmd.creation_flags(CREATE_NEW_CONSOLE);
        }
        cmd.spawn()
            .map(|_child| ()) // no esperamos a que termine
            .map_err(|e| format!("no se pudo ejecutar «{}»: {e}", command.program))
    }
}

#[cfg(windows)]
#[allow(unsafe_code)] // COM y GDI de Win32: cada bloque explica por qué es correcto.
mod platform {
    use base64::Engine;
    use windows::Win32::Foundation::SIZE;
    use windows::Win32::Graphics::Gdi::{
        BI_RGB, BITMAP, BITMAPINFO, BITMAPINFOHEADER, DIB_RGB_COLORS, DeleteObject, GetDC,
        GetDIBits, GetObjectW, HBITMAP, ReleaseDC,
    };
    use windows::Win32::System::Com::{
        COINIT_APARTMENTTHREADED, CoInitializeEx, CoTaskMemFree, CoUninitialize,
    };
    use windows::Win32::UI::Shell::{
        BHID_EnumItems, FOLDERID_AppsFolder, IEnumShellItems, IShellItem, IShellItemImageFactory,
        KF_FLAG_DEFAULT, SHCreateItemFromParsingName, SHGetKnownFolderItem, SIGDN,
        SIGDN_NORMALDISPLAY, SIGDN_PARENTRELATIVEPARSING, SIIGBF_BIGGERSIZEOK, SIIGBF_ICONONLY,
        ShellExecuteW,
    };
    use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;
    use windows::core::{HSTRING, PCWSTR, w};

    use super::InstalledApp;

    /// Tamaño del icono en píxeles (se ve nítido también con escalado al 150 %).
    const ICON_PX: i32 = 48;

    /// Ejecuta `f` en un hilo nuevo con COM inicializado (apartamento STA, el que exige
    /// el Shell). Los hilos de Tauri/tokio no son de COM, así que no se usa el actual.
    fn with_com<T: Send + 'static>(f: impl FnOnce() -> T + Send + 'static) -> Option<T> {
        std::thread::spawn(move || {
            // SAFETY: inicializar y cerrar COM en el mismo hilo, emparejados.
            let init = unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) };
            let result = f();
            if init.is_ok() {
                unsafe { CoUninitialize() };
            }
            result
        })
        .join()
        .ok()
    }

    pub fn list_apps() -> Result<Vec<InstalledApp>, String> {
        with_com(|| -> Result<Vec<InstalledApp>, String> {
            // SAFETY: interfaces COM obtenidas del propio Shell; se liberan solas al salir
            // de ámbito (los tipos de `windows` llaman a Release en Drop).
            unsafe {
                let folder: IShellItem =
                    SHGetKnownFolderItem(&FOLDERID_AppsFolder, KF_FLAG_DEFAULT, None)
                        .map_err(|e| format!("no se pudo abrir la carpeta de aplicaciones: {e}"))?;
                let items: IEnumShellItems = folder
                    .BindToHandler(None, &BHID_EnumItems)
                    .map_err(|e| format!("no se pudieron listar las aplicaciones: {e}"))?;
                let mut apps = Vec::new();
                loop {
                    let mut batch: [Option<IShellItem>; 1] = [None];
                    let mut fetched = 0u32;
                    if items.Next(&mut batch, Some(&mut fetched)).is_err() || fetched == 0 {
                        break;
                    }
                    let Some(item) = batch[0].take() else { break };
                    if let (Some(name), Some(id)) = (
                        display_name(&item, SIGDN_NORMALDISPLAY),
                        display_name(&item, SIGDN_PARENTRELATIVEPARSING),
                    ) {
                        apps.push(InstalledApp { id, name });
                    }
                }
                Ok(apps)
            }
        })
        .unwrap_or_else(|| Err("el hilo de COM falló".into()))
    }

    /// SAFETY: `item` es un IShellItem válido; la cadena devuelta se libera con
    /// CoTaskMemFree, como exige la documentación de GetDisplayName.
    unsafe fn display_name(item: &IShellItem, kind: SIGDN) -> Option<String> {
        unsafe {
            let raw = item.GetDisplayName(kind).ok()?;
            let text = raw.to_string().ok();
            CoTaskMemFree(Some(raw.0 as *const _));
            text
        }
    }

    pub fn icon_data_url(parsing_name: &str) -> Option<String> {
        let name = parsing_name.to_owned();
        with_com(move || -> Option<String> {
            // SAFETY: llamadas COM/GDI con punteros a variables locales que viven durante
            // la llamada; el HBITMAP devuelto por GetImage se libera con DeleteObject.
            unsafe {
                let item: IShellItem =
                    SHCreateItemFromParsingName(&HSTRING::from(name), None).ok()?;
                let factory: IShellItemImageFactory = windows::core::Interface::cast(&item).ok()?;
                let size = SIZE {
                    cx: ICON_PX,
                    cy: ICON_PX,
                };
                let bitmap = factory
                    .GetImage(size, SIIGBF_ICONONLY | SIIGBF_BIGGERSIZEOK)
                    .ok()?;
                let png = bitmap_to_png(bitmap);
                let _ = DeleteObject(bitmap.into());
                let png = png?;
                Some(format!(
                    "data:image/png;base64,{}",
                    base64::engine::general_purpose::STANDARD.encode(png)
                ))
            }
        })
        .flatten()
    }

    /// Copia los píxeles de un HBITMAP y los codifica como PNG.
    ///
    /// SAFETY: `bitmap` es un HBITMAP válido; el búfer tiene exactamente ancho × alto × 4
    /// bytes, que es lo que GetDIBits escribe con 32 bits por píxel.
    unsafe fn bitmap_to_png(bitmap: HBITMAP) -> Option<Vec<u8>> {
        unsafe {
            let mut info = BITMAP::default();
            let got = GetObjectW(
                bitmap.into(),
                std::mem::size_of::<BITMAP>() as i32,
                Some(&mut info as *mut BITMAP as *mut _),
            );
            if got == 0 || info.bmWidth <= 0 || info.bmHeight <= 0 {
                return None;
            }
            let (w, h) = (info.bmWidth, info.bmHeight);
            let mut header = BITMAPINFO {
                bmiHeader: BITMAPINFOHEADER {
                    biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                    biWidth: w,
                    biHeight: -h, // negativo: filas de arriba abajo, como espera PNG
                    biPlanes: 1,
                    biBitCount: 32,
                    biCompression: BI_RGB.0,
                    ..Default::default()
                },
                ..Default::default()
            };
            let mut pixels = vec![0u8; (w * h * 4) as usize];
            let dc = GetDC(None);
            let lines = GetDIBits(
                dc,
                bitmap,
                0,
                h as u32,
                Some(pixels.as_mut_ptr() as *mut _),
                &mut header,
                DIB_RGB_COLORS,
            );
            ReleaseDC(None, dc);
            if lines == 0 {
                return None;
            }
            encode_png(bgra_premultiplied_to_rgba(pixels), w as u32, h as u32)
        }
    }

    pub fn shell_open(target: &str) -> Result<(), String> {
        let target = target.to_owned();
        with_com(move || {
            // SAFETY: cadenas UTF-16 terminadas en NUL que viven durante la llamada.
            let code = unsafe {
                ShellExecuteW(
                    None,
                    w!("open"),
                    &HSTRING::from(target.as_str()),
                    PCWSTR::null(),
                    PCWSTR::null(),
                    SW_SHOWNORMAL,
                )
            };
            // ShellExecute devuelve un valor > 32 si fue bien (herencia de Windows 3.x).
            if code.0 as usize > 32 {
                Ok(())
            } else {
                Err(format!(
                    "Windows no pudo abrir «{target}» (código {})",
                    code.0 as usize
                ))
            }
        })
        .unwrap_or_else(|| Err("el hilo de COM falló".into()))
    }

    pub(super) use super::pixels::{bgra_premultiplied_to_rgba, encode_png};
}

#[cfg(not(windows))]
mod platform {
    use super::InstalledApp;

    pub fn list_apps() -> Result<Vec<InstalledApp>, String> {
        Ok(Vec::new())
    }
    pub fn icon_data_url(_: &str) -> Option<String> {
        None
    }
    pub fn shell_open(_: &str) -> Result<(), String> {
        Err("solo implementado en Windows".into())
    }
}

/// Conversión de píxeles, separada para poder probarla sin Windows.
mod pixels {
    /// Windows da los píxeles en BGRA con alfa *premultiplicado* (cada color ya va
    /// multiplicado por su transparencia). PNG quiere RGBA con alfa normal.
    pub fn bgra_premultiplied_to_rgba(mut px: Vec<u8>) -> Vec<u8> {
        // Iconos antiguos sin canal alfa: todo el alfa a 0. Entonces son opacos.
        let has_alpha = px.as_chunks::<4>().0.iter().any(|p| p[3] != 0);
        for p in px.as_chunks_mut::<4>().0 {
            let (b, g, r, a) = (p[0], p[1], p[2], if has_alpha { p[3] } else { 255 });
            let unpremultiply = |c: u8| {
                if a == 0 {
                    0
                } else {
                    ((u16::from(c) * 255 + u16::from(a) / 2) / u16::from(a)).min(255) as u8
                }
            };
            p.copy_from_slice(&[unpremultiply(r), unpremultiply(g), unpremultiply(b), a]);
        }
        px
    }

    pub fn encode_png(rgba: Vec<u8>, width: u32, height: u32) -> Option<Vec<u8>> {
        let mut out = Vec::new();
        {
            let mut encoder = png::Encoder::new(&mut out, width, height);
            encoder.set_color(png::ColorType::Rgba);
            encoder.set_depth(png::BitDepth::Eight);
            let mut writer = encoder.write_header().ok()?;
            writer.write_image_data(&rgba).ok()?;
        }
        Some(out)
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn converts_premultiplied_bgra() {
            // Rojo puro opaco, y rojo al 50 % premultiplicado (128 de rojo, alfa 128).
            let out = bgra_premultiplied_to_rgba(vec![0, 0, 255, 255, 0, 0, 128, 128]);
            assert_eq!(out, vec![255, 0, 0, 255, 255, 0, 0, 128]);
        }

        #[test]
        fn icons_without_alpha_become_opaque() {
            let out = bgra_premultiplied_to_rgba(vec![10, 20, 30, 0]);
            assert_eq!(out, vec![30, 20, 10, 255]);
        }

        #[test]
        fn fully_transparent_pixels_stay_transparent() {
            let out = bgra_premultiplied_to_rgba(vec![0, 0, 0, 0, 0, 0, 255, 255]);
            assert_eq!(&out[..4], &[0, 0, 0, 0]);
        }

        #[test]
        fn encodes_a_valid_png() {
            let png = encode_png(vec![255; 2 * 2 * 4], 2, 2).unwrap();
            assert_eq!(&png[..8], b"\x89PNG\r\n\x1a\n");
        }
    }
}
