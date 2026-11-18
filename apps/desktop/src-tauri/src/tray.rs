//! Icono en la bandeja del sistema: clic izquierdo abre el lanzador; clic derecho, el menú.

use tauri::AppHandle;
use tauri::image::Image;
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};

use crate::{launcher, settings_window};

pub fn create(app: &AppHandle) -> tauri::Result<()> {
    let open = MenuItem::with_id(app, "open", "Abrir Sift", true, None::<&str>)?;
    let settings = MenuItem::with_id(app, "settings", "Ajustes…", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Salir", true, None::<&str>)?;
    let separator = PredefinedMenuItem::separator(app)?;
    let menu = Menu::with_items(app, &[&open, &settings, &separator, &quit])?;

    let mut tray = TrayIconBuilder::with_id("main")
        .tooltip("Sift")
        .menu(&menu)
        .show_menu_on_left_click(false) // el menú, solo con clic derecho
        .on_menu_event(|app, event| match event.id.as_ref() {
            "open" => launcher::show(app),
            "settings" => settings_window::open(app, false, crate::settings_window::Origin::Tray),
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            // Al soltar el botón (no al pulsarlo), como el resto de iconos de Windows.
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                launcher::show(tray.app_handle());
            }
        });
    tray = tray.icon(icon_for(crate::appearance::taskbar_theme()));
    tray.build(app)?;
    Ok(())
}

/// El icono de la bandeja es de un solo color, como los de Windows: blanco sobre la barra de
/// tareas oscura y casi negro sobre la clara (`icons/source/tray-*.svg`).
fn icon_for(taskbar: crate::appearance::Theme) -> Image<'static> {
    match taskbar {
        crate::appearance::Theme::Dark => tauri::include_image!("icons/tray/tray-white.png"),
        crate::appearance::Theme::Light => tauri::include_image!("icons/tray/tray-black.png"),
    }
}

/// Vuelve a elegir el icono por si ha cambiado el tema de la barra de tareas. Es barato (una
/// lectura del registro): se llama al mostrar la barra.
pub fn refresh_icon(app: &AppHandle) {
    if let Some(tray) = app.tray_by_id("main")
        && let Err(e) = tray.set_icon(Some(icon_for(crate::appearance::taskbar_theme())))
    {
        tracing::debug!(error = %e, "no se pudo cambiar el icono de la bandeja");
    }
}
