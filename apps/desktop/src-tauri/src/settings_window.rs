//! Ventana de ajustes. Al revés que el lanzador, se crea solo cuando se abre y se
//! destruye al cerrarla: se usa poco y así no ocupa memoria el resto del tiempo.
//!
//! Se crea **oculta** y solo se muestra cuando la UI avisa de que ya ha pintado
//! (`settings_ready`): así el primer fotograma que se ve ya tiene el contenido y el
//! vidrio, y no una ventana vacía y transparente.

use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder};
use tauri_specta::Event;

use crate::commands::NewSpaceRequested;
use crate::materials;

pub const LABEL: &str = "settings";

/// Tiempo máximo que la ventana espera oculta a que la UI pinte.
const REVEAL_TIMEOUT: Duration = Duration::from_millis(1500);

/// Muestra la ventana de ajustes si aún está oculta (la UI ya ha pintado).
pub fn reveal(app: &AppHandle) {
    let Some(window) = app.get_webview_window(LABEL) else {
        return;
    };
    if window.is_visible().unwrap_or(false) {
        return;
    }
    if let Err(e) = window.show().and_then(|()| window.set_focus()) {
        tracing::warn!(error = %e, "no se pudo mostrar la ventana de ajustes");
    }
    // Windows 10 no pinta bien el Acrylic aplicado a una ventana que aún no se había
    // mostrado (queda mucho más transparente hasta que se mueve): se reaplica ya visible.
    materials::refresh(app, &window);
}

/// Desde dónde se abrieron los ajustes la última vez. Decide qué pasa al cerrarlos.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Origin {
    /// Desde la barra (Ctrl+, o «Ajustes»): al cerrar, la barra vuelve, con el foco en
    /// la búsqueda. Lo normal es que hayas cambiado algo y quieras usarlo.
    Launcher,
    /// Desde el icono de la bandeja: al cerrar no aparece nada.
    Tray,
}

/// ¿Hay que volver a la barra al cerrar los ajustes? (Tauri lo guarda con `manage`.)
#[derive(Default)]
pub struct ReturnToLauncher(AtomicBool);

/// La ventana de ajustes se ha cerrado: si se abrió desde la barra, la barra vuelve.
pub fn on_closed(app: &AppHandle) {
    if app
        .state::<ReturnToLauncher>()
        .0
        .swap(false, Ordering::Relaxed)
    {
        crate::launcher::show(app);
    }
}

/// Abre los ajustes. Con `new_space`, directamente en "crear Espacio".
pub fn open(app: &AppHandle, new_space: bool, origin: Origin) {
    open_page(app, new_space, false, origin);
}

/// Abre los ajustes en modo bienvenida (la primera vez que arranca Sift).
pub fn open_welcome(app: &AppHandle) {
    open_page(app, false, true, Origin::Tray);
}

fn open_page(app: &AppHandle, new_space: bool, welcome: bool, origin: Origin) {
    // Cuenta el último sitio desde el que se pidieron, también si ya estaban abiertos.
    app.state::<ReturnToLauncher>()
        .0
        .store(origin == Origin::Launcher, Ordering::Relaxed);
    // Si ya está abierta, se trae al frente (y se le avisa si hay que crear un Espacio).
    if let Some(window) = app.get_webview_window(LABEL) {
        let _ = window.unminimize();
        let _ = window.set_focus();
        if new_space {
            let _ = NewSpaceRequested.emit(app);
        }
        return;
    }
    // Si no, se crea; el parámetro de la URL le dice que empiece creando un Espacio.
    let page = if welcome {
        "settings.html?welcome=1"
    } else if new_space {
        "settings.html?new=1"
    } else {
        "settings.html"
    };
    let built = WebviewWindowBuilder::new(app, LABEL, WebviewUrl::App(page.into()))
        .title("Ajustes de Sift")
        .inner_size(920.0, 680.0)
        .min_inner_size(760.0, 520.0)
        .center()
        // Transparente para que se vea el material de Windows (el vidrio) detrás.
        .transparent(true)
        // Sin barra de título nativa: en Windows 10 es negra y no admite vidrio. La pinta
        // la UI (`TitleBar.tsx`); el arrastre, el doble clic y el ajuste a los bordes
        // siguen siendo los de Windows porque el arrastre lo inicia el sistema.
        .decorations(false)
        .shadow(true)
        // Oculta hasta que tenga el vidrio: si no, el primer fotograma sale sin tinte.
        .visible(false)
        .build();
    match built {
        Ok(window) => {
            // El mismo vidrio que la barra, salvo con «Desactivar vidrio» o si Windows
            // no puede pintarlo bien.
            materials::sync(app, &window);
            // Por si la UI no llegara a avisar (un error al cargar): se muestra igualmente.
            let app = app.clone();
            std::thread::spawn(move || {
                std::thread::sleep(REVEAL_TIMEOUT);
                let handle = app.clone();
                let _ = app.run_on_main_thread(move || reveal(&handle));
            });
        }
        Err(e) => tracing::error!(error = %e, "no se pudo abrir la ventana de ajustes"),
    }
}
