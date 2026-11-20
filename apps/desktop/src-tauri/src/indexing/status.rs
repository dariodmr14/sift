//! Lo que el indexado cuenta al resto de la app: su estado, las fases y el evento de progreso.

use serde::{Deserialize, Serialize};
use tauri_specta::Event;

/// En qué está el indexado.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum IndexPhase {
    /// Sin trabajo: todo al día.
    #[default]
    Idle,
    /// Recorriendo las carpetas para ver qué ha cambiado («Buscando cambios…»).
    Scanning,
    /// Leyendo lo nuevo o cambiado («Leyendo `done` de `total` archivos»).
    Reading,
    /// Leyendo, pero en pausa porque el PC está ocupado.
    Paused,
    /// Poniendo al día unos pocos archivos que acaban de cambiar.
    Updating,
    /// Descargando el modelo de la búsqueda inteligente (la primera vez).
    Downloading,
    /// Calculando vectores («Preparando búsqueda inteligente · `done` de `total`»).
    Embedding,
}

/// Estado del indexado, para Ajustes → Carpetas.
#[derive(Debug, Clone, Default, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct IndexStatus {
    pub running: bool,
    pub phase: IndexPhase,
    /// Carpeta que se está recorriendo o leyendo.
    pub current_root: Option<String>,
    /// Buscando cambios: archivos vistos hasta ahora.
    pub scanned: u32,
    /// Leyendo: archivos ya leídos de `total`.
    pub done: u32,
    /// Leyendo: archivos nuevos o cambiados que hay que leer en esta pasada.
    pub total: u32,
    /// Resumen de la última revisión completa.
    pub last: Option<LastRun>,
    pub error: Option<String>,
    /// ¿Se vigilan las carpetas en tiempo real? (si no, solo se revisan al arrancar)
    pub watching: bool,
    /// ¿Se lee el contenido de los PDF? (hace falta `pdfium.dll`)
    pub reads_pdf: bool,
    /// Búsqueda inteligente: activada, lista (modelo cargado) y, si algo va mal, por qué.
    pub smart: SmartStatus,
}

impl IndexStatus {
    /// Un trabajo ha terminado (revisión completa o cambios recientes): al día, y todo lo
    /// que había que leer, leído. Sin esto, tras unos cambios el estado decía «0 de 7» en
    /// reposo, y quien leyera el progreso creería que faltan archivos.
    pub(super) fn finish(&mut self) {
        self.running = false;
        self.phase = IndexPhase::Idle;
        self.current_root = None;
        self.done = self.total;
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct SmartStatus {
    pub enabled: bool,
    /// Modelo cargado: la búsqueda ya usa el significado (aunque falten vectores).
    pub ready: bool,
    /// Vectores calculados y los que tendría que haber.
    pub done: u32,
    pub total: u32,
    /// Descarga del modelo, en MB.
    pub downloaded_mb: u32,
    pub download_mb: u32,
    /// Falta el «sí» del usuario para descargar el modelo: la UI enseña el botón.
    pub awaiting_permission: bool,
    /// Por qué no está lista (sin conexión, etc.), en palabras.
    pub note: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct LastRun {
    /// Archivos en todas las carpetas al acabar.
    pub files: u32,
    pub added: u32,
    pub updated: u32,
    pub unchanged: u32,
    pub removed: u32,
    pub errors: u32,
    pub seconds: f64,
}

/// Lo que dice Ajustes si un trabajo del indexado falla de forma inesperada.
pub(super) const PANIC_NOTE: &str = "Algo falló al revisar tus carpetas. Se volverá a intentar con el próximo cambio o al abrir Sift.";
/// Lo que dice si el hilo de indexado se ha parado del todo.
pub(super) const STOPPED_NOTE: &str =
    "El indexado se ha parado por un fallo. Cierra Sift y vuelve a abrirlo.";

/// Evento: el estado del indexado ha cambiado.
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type, Event)]
pub struct IndexProgress(pub IndexStatus);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_finished_run_is_idle_with_everything_read() {
        let mut s = IndexStatus {
            running: true,
            phase: IndexPhase::Updating,
            current_root: Some("C:\\Notas".into()),
            done: 0,
            total: 7,
            ..IndexStatus::default()
        };
        s.finish();
        assert!(!s.running);
        assert_eq!(s.phase, IndexPhase::Idle);
        assert_eq!(s.current_root, None);
        assert_eq!((s.done, s.total), (7, 7));
    }
}
