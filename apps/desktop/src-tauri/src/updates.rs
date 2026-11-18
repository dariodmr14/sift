//! Actualizaciones automáticas (ADR 0018). **Preparadas e inactivas:** solo funcionan en
//! una compilación que lleve la configuración del actualizador (la clave pública y la
//! dirección del `latest.json`), que añade el workflow de Releases con
//! `--config src-tauri/tauri.updater.conf.json`. La compilación de todos los días no la
//! lleva: el plugin ni se registra y Ajustes no enseña la opción.
//!
//! Con la opción activada (apagada por defecto, porque es una conexión a internet), al
//! arrancar se mira si hay una versión nueva. Si la hay, un diálogo de Windows pregunta
//! antes de descargarla. La actualización va firmada: el plugin comprueba la firma con la
//! clave pública antes de instalar nada.

use tauri::AppHandle;
use tauri_plugin_dialog::{DialogExt, MessageDialogButtons, MessageDialogKind};
use tauri_plugin_updater::UpdaterExt;

/// ¿Lleva esta compilación la configuración del actualizador?
pub fn configured(app: &AppHandle) -> bool {
    app.config().plugins.0.contains_key("updater")
}

/// Registra el actualizador (si está configurado) y, si el usuario lo ha activado, mira en
/// segundo plano si hay una versión nueva.
pub fn init(app: &AppHandle) {
    if !configured(app) {
        return;
    }
    if let Err(e) = app.plugin(tauri_plugin_updater::Builder::new().build()) {
        tracing::warn!(error = %e, "no se pudo preparar el actualizador");
        return;
    }
    if !crate::load_settings(app).updates.check_on_startup {
        return;
    }
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        if let Err(e) = check(&app).await {
            // Sin conexión, GitHub caído…: no es un fallo de Sift. Se reintenta al
            // siguiente arranque.
            tracing::info!(error = %e, "no se pudo comprobar si hay actualizaciones");
        }
    });
}

async fn check(app: &AppHandle) -> anyhow::Result<()> {
    let Some(update) = app.updater()?.check().await? else {
        tracing::info!("Sift está al día");
        return Ok(());
    };
    let message = format!(
        "Hay una versión nueva de Sift: {} (tienes la {}).\n\n\
         ¿Descargarla e instalarla ahora? Sift se cerrará y se volverá a abrir.",
        update.version,
        sift_core::VERSION
    );
    let ask = app.clone();
    let yes = tauri::async_runtime::spawn_blocking(move || {
        ask.dialog()
            .message(message)
            .title("Sift: actualización")
            .kind(MessageDialogKind::Info)
            .buttons(MessageDialogButtons::OkCancelCustom(
                "Instalar y reiniciar".into(),
                "Ahora no".into(),
            ))
            .blocking_show()
    })
    .await?;
    if !yes {
        tracing::info!(version = %update.version, "actualización pospuesta");
        return Ok(());
    }
    update.download_and_install(|_, _| {}, || {}).await?;
    tracing::info!(version = %update.version, "actualización instalada; reiniciando");
    app.restart();
}
