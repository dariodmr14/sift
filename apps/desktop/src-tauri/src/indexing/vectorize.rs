//! La búsqueda inteligente: descargar y cargar el modelo, abrir el índice de vectores y
//! calcular los vectores que falten sin molestar.

use std::sync::Arc;
use std::sync::mpsc::Receiver;
use std::time::Instant;

use sift_extract::Extractor;
use tauri::Manager;

use super::Job;
use super::status::{IndexPhase, SmartStatus};
use super::worker::{Worker, drain};

/// Fragmentos que se vectorizan de una vez: pocos, para ceder el paso enseguida.
const EMBED_BATCH: usize = 8;
/// Cada cuántos vectores nuevos se guarda el índice HNSW en disco.
const SAVE_VECTORS_EVERY: usize = 2_000;

impl Worker {
    /// Un rato de trabajo de búsqueda inteligente: prepararla si no lo está (descargar y
    /// cargar el modelo, cargar o reconstruir el índice de vectores) y vectorizar lo que
    /// falte. Cede el paso en cuanto llega otro trabajo.
    pub(super) fn smart_work(&mut self, extractor: &Extractor, rx: &Receiver<Job>) -> SmartOutcome {
        let done = SmartOutcome {
            more: false,
            sync: false,
        };
        let card = sift_embeddings::MODELS[0];
        let enabled = !crate::load_settings(&self.app)
            .indexing
            .disable_smart_search;
        if !enabled {
            self.smart.set(None);
            self.embedder = None;
            self.target_vectors = None;
            self.update(|s| s.smart = SmartStatus::default());
            return done;
        }
        let missing = self.embedder.is_none() || self.target_vectors.is_none();
        if missing && self.awaiting_permission(card) {
            // No es un error: es lo normal hasta que la persona acepte la descarga. El
            // siguiente `smart_changed()` (al aceptar) lo vuelve a intentar.
            tracing::info!("búsqueda inteligente a la espera del permiso para descargar");
            let total_mb = (card.download_bytes() / 1_000_000) as u32;
            self.update(|s| {
                s.running = false;
                s.phase = IndexPhase::Idle;
                s.smart = SmartStatus {
                    enabled: true,
                    awaiting_permission: true,
                    download_mb: total_mb,
                    ..SmartStatus::default()
                };
            });
            return done;
        }
        if missing && let Err(note) = self.prepare_smart(card) {
            tracing::warn!(%note, "búsqueda inteligente no disponible por ahora");
            self.update(|s| {
                s.running = false;
                s.phase = IndexPhase::Idle;
                s.smart = SmartStatus {
                    enabled: true,
                    note: Some(note),
                    ..SmartStatus::default()
                };
            });
            // Se reintenta en el siguiente trabajo (p. ej. al volver la conexión y
            // revisar las carpetas al arrancar).
            return done;
        }
        self.embed_pending(card, extractor, rx)
    }

    /// ¿Falta el modelo y la persona aún no ha dicho que sí a descargarlo?
    fn awaiting_permission(&self, card: &'static sift_embeddings::ModelCard) -> bool {
        let Ok(app_data) = self.app.path().app_data_dir() else {
            return false;
        };
        let dir = crate::models::model_dir(&app_data, card);
        needs_permission(
            crate::models::is_ready(&dir, card),
            crate::load_settings(&self.app)
                .indexing
                .model_download_allowed,
        )
    }

    /// Descarga el modelo si falta, lo carga (dos veces: indexado y consultas) y carga o
    /// reconstruye el índice de vectores. `Err` con el motivo en palabras.
    fn prepare_smart(&mut self, card: &'static sift_embeddings::ModelCard) -> Result<(), String> {
        let app_data = self.app.path().app_data_dir().map_err(|e| e.to_string())?;
        let dir = crate::models::model_dir(&app_data, card);
        if !crate::models::is_ready(&dir, card) {
            let total_mb = (card.download_bytes() / 1_000_000) as u32;
            self.update(|s| {
                s.running = true;
                s.phase = IndexPhase::Downloading;
                s.smart = SmartStatus {
                    enabled: true,
                    download_mb: total_mb,
                    ..SmartStatus::default()
                };
            });
            let started = Instant::now();
            let cancel = self.cancel.clone();
            let result = crate::models::download(&dir, card, &cancel, |done, _| {
                self.update_throttled(|s| s.smart.downloaded_mb = (done / 1_000_000) as u32);
            });
            if let Err(e) = result {
                // El detalle técnico, al log; a la persona, qué pasa y qué hará Sift.
                tracing::warn!(error = %e, "no se pudo descargar la búsqueda inteligente");
                return Err(match e {
                    crate::models::DownloadError::Network(_) => {
                        "Sin conexión: se descargará cuando la haya.".to_owned()
                    }
                    crate::models::DownloadError::Corrupt(_) => {
                        "La descarga llegó dañada: se volverá a intentar.".to_owned()
                    }
                    crate::models::DownloadError::Io(_) => {
                        "No se pudo guardar la descarga. ¿Queda espacio en el disco?".to_owned()
                    }
                    crate::models::DownloadError::Cancelled => "Descarga cancelada.".to_owned(),
                });
            }
            tracing::info!(
                model = card.id,
                secs = started.elapsed().as_secs_f64(),
                "modelo de búsqueda inteligente descargado"
            );
        }
        let load = |threads| {
            sift_embeddings::OnnxEmbedder::load(card, &dir, threads).map_err(|e| {
                tracing::warn!(error = %e, "no se pudo cargar el modelo");
                "No se pudo poner en marcha. Prueba a desactivarla y volver a activarla \
                 en Ajustes → General."
                    .to_owned()
            })
        };
        let background = load(crate::smart::BACKGROUND_THREADS)?;
        let vectors = Arc::new(self.open_vectors(&app_data, card)?);

        // ¿Con qué modelo se busca? Con el actual si ya lo cubre todo o si no hay otro;
        // si se está cambiando de modelo, con el anterior hasta que el nuevo acabe
        // (ADR 0009, punto 7): nunca se queda sin búsqueda por significado.
        let (done, total, others) = {
            let db = self.db();
            let db = db.0.lock().unwrap_or_else(|p| p.into_inner());
            let (done, total) = db
                .embedding_progress(card.id, &sift_storage::EmbedPolicy::standard())
                .map_err(|e| e.to_string())?;
            (
                done,
                total,
                db.embedding_models().map_err(|e| e.to_string())?,
            )
        };
        let previous = others
            .iter()
            .filter(|(id, _)| id != card.id)
            .filter_map(|(id, _)| sift_embeddings::model(id))
            .find(|old| crate::models::is_ready(&crate::models::model_dir(&app_data, old), old));
        let ready = match previous {
            Some(old) if done < total => {
                tracing::info!(
                    old = old.id,
                    new = card.id,
                    "cambio de modelo: se busca con el anterior mientras tanto"
                );
                let old_dir = crate::models::model_dir(&app_data, old);
                crate::smart::Ready {
                    card: old,
                    query: Arc::new(
                        sift_embeddings::OnnxEmbedder::load(
                            old,
                            &old_dir,
                            crate::smart::QUERY_THREADS,
                        )
                        .map_err(|e| e.to_string())?,
                    ),
                    vectors: Arc::new(self.open_vectors(&app_data, old)?),
                }
            }
            _ => crate::smart::Ready {
                card,
                query: Arc::new(load(crate::smart::QUERY_THREADS)?),
                vectors: vectors.clone(),
            },
        };
        self.embedder = Some(background);
        self.target_vectors = Some(vectors);
        self.smart.set(Some(ready));
        tracing::info!(model = card.id, "búsqueda inteligente lista");
        Ok(())
    }

    /// El índice de vectores de un modelo: el guardado si cuadra con SQLite; si no, se
    /// reconstruye desde SQLite (segundos: no se recalcula nada).
    fn open_vectors(
        &self,
        app_data: &std::path::Path,
        card: &sift_embeddings::ModelCard,
    ) -> Result<sift_search::VectorIndex, String> {
        let path = crate::smart::vectors_path(app_data, card);
        let db = self.db();
        let stored = {
            let db = db.0.lock().unwrap_or_else(|p| p.into_inner());
            db.count_embeddings(card.id).map_err(|e| e.to_string())?
        };
        if let Some(v) = sift_search::VectorIndex::load(&path, card.dims)
            && v.len() as u64 == stored
        {
            return Ok(v);
        }
        let started = Instant::now();
        let v = sift_search::VectorIndex::new(card.dims).map_err(|e| e.to_string())?;
        // Con el candado solo mientras se leen los vectores; construir el índice y guardarlo
        // (lo lento) va sin él, para no parar la vista previa ni los Ajustes (auditoría 1.0).
        let mut batch = Vec::new();
        {
            let db = db.0.lock().unwrap_or_else(|p| p.into_inner());
            db.for_each_embedding(card.id, |h, vector| batch.push((h, vector)))
                .map_err(|e| e.to_string())?;
        }
        v.add(&batch).map_err(|e| e.to_string())?;
        drop(batch);
        if let Err(e) = v.save(&path) {
            tracing::warn!(error = %e, "no se pudo guardar el índice de vectores");
        }
        tracing::info!(
            model = card.id,
            vectors = v.len(),
            secs = started.elapsed().as_secs_f64(),
            "índice de vectores reconstruido desde SQLite"
        );
        Ok(v)
    }

    /// Vectoriza lo que falte, en lotes pequeños, cediendo el paso entre lote y lote.
    fn embed_pending(
        &mut self,
        card: &'static sift_embeddings::ModelCard,
        extractor: &Extractor,
        rx: &Receiver<Job>,
    ) -> SmartOutcome {
        let policy = sift_storage::EmbedPolicy::standard();
        let Some(target) = self.target_vectors.clone() else {
            return SmartOutcome {
                more: false,
                sync: false,
            };
        };
        let progress = |worker: &Worker| {
            let db = worker.db();
            let db = db.0.lock().unwrap_or_else(|p| p.into_inner());
            db.embedding_progress(card.id, &policy).unwrap_or((0, 0))
        };
        let (mut done, total) = progress(self);
        let mut sync = false;
        let mut last_page = Vec::new();
        loop {
            if last_page.is_empty() {
                let page = {
                    let db = self.db();
                    let db = db.0.lock().unwrap_or_else(|p| p.into_inner());
                    db.pending_texts(card.id, &policy, 64)
                };
                match page {
                    Ok(mut page) if !page.is_empty() => {
                        // Longitudes parecidas en cada lote: menos relleno que calcular.
                        page.sort_by_key(|(_, t)| std::cmp::Reverse(t.len()));
                        last_page = page;
                    }
                    Ok(_) => break,
                    Err(e) => {
                        tracing::error!(error = %e, "no se pudo leer qué falta por vectorizar");
                        break;
                    }
                }
            }
            let batch: Vec<(i64, String)> =
                last_page.split_off(last_page.len().saturating_sub(EMBED_BATCH));
            let worked = Instant::now();
            let texts: Vec<&str> = batch.iter().map(|(_, t)| t.as_str()).collect();
            let Some(embedder) = &self.embedder else {
                break;
            };
            let vectors = match sift_embeddings::Embedder::embed(
                embedder,
                &texts,
                sift_embeddings::Purpose::Document,
            ) {
                Ok(v) => v,
                Err(e) => {
                    tracing::error!(error = %e, "no se pudieron calcular vectores");
                    break;
                }
            };
            let rows: Vec<(i64, Vec<f32>)> = batch.iter().map(|(h, _)| *h).zip(vectors).collect();
            let saved = {
                let db = self.db();
                let mut db = db.0.lock().unwrap_or_else(|p| p.into_inner());
                db.save_embeddings(card.id, &rows)
            };
            if let Err(e) = saved {
                tracing::error!(error = %e, "no se pudieron guardar vectores");
                break;
            }
            if let Err(e) = target.add(&rows) {
                tracing::error!(error = %e, "no se pudieron añadir vectores al índice");
            }
            done += rows.len() as u64;
            self.unsaved_vectors += rows.len();
            if self.unsaved_vectors >= SAVE_VECTORS_EVERY {
                self.save_vectors(card, &target);
            }
            self.update_throttled(|s| {
                s.running = true;
                s.phase = IndexPhase::Embedding;
                s.done = done.min(total) as u32;
                s.total = total as u32;
                s.smart.enabled = true;
                s.smart.ready = true;
                s.smart.done = done.min(total) as u32;
                s.smart.total = total as u32;
                s.smart.note = None;
            });
            // Cede el paso: cambios recientes al momento; una revisión, saliendo de aquí.
            let before = self.status.lock().unwrap_or_else(|p| p.into_inner()).phase;
            let (paths, sync_too) = drain(rx, Vec::new());
            if !paths.is_empty() && self.changed(extractor, paths).is_err() {
                sync = true;
            }
            if sync_too || sync {
                sync = true;
                break;
            }
            if self.status.lock().unwrap_or_else(|p| p.into_inner()).phase != before {
                self.update(|s| s.phase = IndexPhase::Embedding);
            }
            // Mismo descanso y misma pausa que el indexado.
            if self.pace(worked.elapsed()) {
                sync = true;
                break;
            }
        }
        // Al terminar (o al ceder): limpiar vectores huérfanos y guardar el índice.
        let finished = !sync;
        if finished {
            let pruned = {
                let db = self.db();
                let mut db = db.0.lock().unwrap_or_else(|p| p.into_inner());
                db.prune_embeddings(card.id, &policy)
            };
            match pruned {
                Ok(gone) if !gone.is_empty() => {
                    let _ = target.remove(&gone);
                    self.unsaved_vectors += gone.len();
                }
                Ok(_) => {}
                Err(e) => tracing::warn!(error = %e, "no se pudieron limpiar vectores"),
            }
        }
        if self.unsaved_vectors > 0 {
            self.save_vectors(card, &target);
        }
        let (done, total) = progress(self);
        // Cambio de modelo terminado: las consultas pasan al nuevo y se borra el anterior.
        if done >= total
            && let Some(current) = self.smart.ready()
            && current.card.id != card.id
        {
            self.switch_queries_to(card, target.clone());
        }
        self.update(|s| {
            if matches!(s.phase, IndexPhase::Embedding) {
                s.running = false;
                s.phase = IndexPhase::Idle;
            }
            s.smart = SmartStatus {
                enabled: true,
                ready: true,
                done: done as u32,
                total: total as u32,
                ..SmartStatus::default()
            };
        });
        SmartOutcome {
            more: sync && done < total,
            sync,
        }
    }

    /// El modelo nuevo ya lo cubre todo: las consultas pasan a usarlo y se borran los
    /// vectores (y el índice) del anterior.
    fn switch_queries_to(
        &mut self,
        card: &'static sift_embeddings::ModelCard,
        vectors: Arc<sift_search::VectorIndex>,
    ) {
        let Ok(app_data) = self.app.path().app_data_dir() else {
            return;
        };
        let dir = crate::models::model_dir(&app_data, card);
        let query =
            match sift_embeddings::OnnxEmbedder::load(card, &dir, crate::smart::QUERY_THREADS) {
                Ok(q) => q,
                Err(e) => {
                    tracing::error!(error = %e, "no se pudo cargar el modelo nuevo para buscar");
                    return;
                }
            };
        let old = self.smart.ready().map(|r| r.card);
        self.smart.set(Some(crate::smart::Ready {
            card,
            query: Arc::new(query),
            vectors,
        }));
        let dropped = {
            let db = self.db();
            let mut db = db.0.lock().unwrap_or_else(|p| p.into_inner());
            db.drop_other_models(card.id)
        };
        if let Some(old) = old {
            let _ = std::fs::remove_file(crate::smart::vectors_path(&app_data, old));
        }
        tracing::info!(new = card.id, dropped = ?dropped.ok(), "modelo cambiado");
    }

    fn save_vectors(
        &mut self,
        card: &sift_embeddings::ModelCard,
        vectors: &sift_search::VectorIndex,
    ) {
        let Ok(app_data) = self.app.path().app_data_dir() else {
            return;
        };
        match vectors.save(&crate::smart::vectors_path(&app_data, card)) {
            Ok(()) => self.unsaved_vectors = 0,
            Err(e) => tracing::warn!(error = %e, "no se pudo guardar el índice de vectores"),
        }
    }
}

/// Qué ha pasado en un rato de trabajo de búsqueda inteligente.
pub(super) struct SmartOutcome {
    /// Queda trabajo (se ha cedido el paso a otra cosa).
    pub(super) more: bool,
    /// Alguien ha pedido una revisión completa mientras tanto.
    pub(super) sync: bool,
}

/// El modelo nunca se descarga sin un «sí» explícito; si ya está en disco, se usa igual.
fn needs_permission(model_ready: bool, allowed: bool) -> bool {
    !model_ready && !allowed
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn download_needs_an_explicit_yes() {
        assert!(needs_permission(false, false));
        assert!(!needs_permission(false, true));
        assert!(!needs_permission(true, false));
        assert!(!needs_permission(true, true));
    }
}
