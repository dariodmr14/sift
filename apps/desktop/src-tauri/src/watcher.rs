//! Vigilante de carpetas: avisa en cuanto algo cambia dentro de las carpetas indexadas,
//! para que editar, crear, borrar, renombrar o mover un archivo se vea en la búsqueda en
//! segundos, sin esperar a la siguiente revisión completa.
//!
//! Usa el crate `notify` (en Windows, `ReadDirectoryChangesW`). Windows avisa de cada
//! cambio varias veces y a ráfagas (guardar un documento son 3 o 4 avisos; descomprimir
//! un zip, miles), así que no se reacciona a cada uno: las rutas se **juntan** en un
//! [`ChangeBuffer`] y se entregan de golpe cuando llevan un momento quietas.
//!
//! Qué hacer con esas rutas (leer, borrar, mover) lo decide `sift_indexer::sync_paths`.
//! La revisión completa al arrancar sigue existiendo: cubre lo que cambió con Sift cerrado.

use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, Weak};
use std::time::{Duration, Instant};

use notify::{EventKind, RecommendedWatcher, RecursiveMode, Watcher};

/// Tiempo sin avisos nuevos tras el que se entregan los cambios.
const QUIET: Duration = Duration::from_millis(400);
/// Aunque sigan llegando avisos (una copia larga), se entrega al menos cada tanto.
const MAX_WAIT: Duration = Duration::from_secs(3);
/// Cada cuánto se mira si hay cambios listos.
const TICK: Duration = Duration::from_millis(100);

/// Rutas cambiadas, acumuladas hasta que se calman.
#[derive(Default)]
pub struct ChangeBuffer {
    paths: HashSet<PathBuf>,
    first: Option<Instant>,
    last: Option<Instant>,
}

impl ChangeBuffer {
    pub fn push(&mut self, paths: impl IntoIterator<Item = PathBuf>, now: Instant) {
        let before = self.paths.len();
        self.paths.extend(paths);
        if self.paths.len() > before || self.first.is_none() {
            self.first.get_or_insert(now);
            self.last = Some(now);
        }
    }

    /// Las rutas acumuladas, si llevan `QUIET` sin novedades o `MAX_WAIT` esperando.
    pub fn take_ready(&mut self, now: Instant) -> Option<Vec<PathBuf>> {
        let (first, last) = (self.first?, self.last?);
        if self.paths.is_empty() {
            return None;
        }
        if now.duration_since(last) < QUIET && now.duration_since(first) < MAX_WAIT {
            return None;
        }
        self.first = None;
        self.last = None;
        Some(self.paths.drain().collect())
    }
}

/// Vigila un conjunto de carpetas y llama a `on_changes` con las rutas que cambian.
pub struct FolderWatcher {
    watcher: RecommendedWatcher,
    watched: Vec<PathBuf>,
    /// Mantiene vivo el hilo que entrega los cambios (se para al soltar el vigilante).
    _buffer: Arc<Mutex<ChangeBuffer>>,
}

impl FolderWatcher {
    pub fn start(
        on_changes: impl Fn(Vec<PathBuf>) + Send + 'static,
    ) -> notify::Result<FolderWatcher> {
        let buffer = Arc::new(Mutex::new(ChangeBuffer::default()));
        let sink = buffer.clone();
        let watcher = notify::recommended_watcher(move |event: notify::Result<notify::Event>| {
            match event {
                // Leer un archivo no lo cambia.
                Ok(event) if matches!(event.kind, EventKind::Access(_)) => {}
                Ok(event) => sink
                    .lock()
                    .unwrap_or_else(|p| p.into_inner())
                    .push(event.paths, Instant::now()),
                Err(e) => tracing::warn!(error = %e, "aviso de cambios con error"),
            }
        })?;

        // Hilo que entrega los cambios cuando se calman. Termina cuando el vigilante se
        // suelta (ya no queda ninguna referencia fuerte al buffer).
        let weak: Weak<Mutex<ChangeBuffer>> = Arc::downgrade(&buffer);
        std::thread::Builder::new()
            .name("sift-watcher".into())
            .spawn(move || {
                while let Some(buffer) = weak.upgrade() {
                    let ready = buffer
                        .lock()
                        .unwrap_or_else(|p| p.into_inner())
                        .take_ready(Instant::now());
                    drop(buffer);
                    if let Some(paths) = ready {
                        on_changes(paths);
                    }
                    std::thread::sleep(TICK);
                }
            })
            .map_err(notify::Error::io)?;

        Ok(FolderWatcher {
            watcher,
            watched: Vec::new(),
            _buffer: buffer,
        })
    }

    /// Deja vigiladas exactamente estas carpetas (con todo lo que tienen dentro). Las que
    /// no existen ahora mismo (un disco desconectado) se saltan.
    pub fn watch(&mut self, roots: &[PathBuf]) {
        for gone in self.watched.iter().filter(|w| !roots.contains(w)) {
            let _ = self.watcher.unwatch(gone);
        }
        let mut watched = Vec::new();
        for root in roots {
            if self.watched.contains(root) {
                watched.push(root.clone());
                continue;
            }
            match self.watcher.watch(root, RecursiveMode::Recursive) {
                Ok(()) => {
                    tracing::info!(root = %root.display(), "carpeta vigilada");
                    watched.push(root.clone());
                }
                Err(e) => {
                    tracing::warn!(root = %root.display(), error = %e, "no se pudo vigilar la carpeta")
                }
            }
        }
        self.watched = watched;
    }

    #[cfg(test)]
    pub fn watched(&self) -> &[PathBuf] {
        &self.watched
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc;

    #[test]
    fn changes_are_delivered_once_they_settle() {
        let mut buffer = ChangeBuffer::default();
        let t0 = Instant::now();
        buffer.push([PathBuf::from("a.md")], t0);
        buffer.push(
            [PathBuf::from("a.md"), PathBuf::from("b.md")],
            t0 + QUIET / 2,
        );
        // Aún llegan avisos: se espera.
        assert_eq!(buffer.take_ready(t0 + QUIET), None);
        // Quieto el tiempo suficiente: se entregan juntos y sin repetir.
        let mut ready = buffer.take_ready(t0 + QUIET * 2).unwrap();
        ready.sort();
        assert_eq!(ready, [PathBuf::from("a.md"), PathBuf::from("b.md")]);
        assert_eq!(buffer.take_ready(t0 + QUIET * 9), None); // ya entregado
    }

    #[test]
    fn a_long_burst_is_delivered_anyway() {
        let mut buffer = ChangeBuffer::default();
        let t0 = Instant::now();
        // Una copia larga: un aviso cada 100 ms, nunca hay silencio.
        let mut t = t0;
        let mut n = 0;
        while t.duration_since(t0) < MAX_WAIT {
            buffer.push([PathBuf::from(format!("f{n}.md"))], t);
            assert_eq!(buffer.take_ready(t), None);
            t += Duration::from_millis(100);
            n += 1;
        }
        assert_eq!(buffer.take_ready(t).map(|p| p.len()), Some(n));
    }

    /// Espera a que lleguen cambios que cumplan `matches`, juntando entregas.
    fn wait_for(
        rx: &mpsc::Receiver<Vec<PathBuf>>,
        matches: impl Fn(&HashSet<PathBuf>) -> bool,
    ) -> HashSet<PathBuf> {
        let deadline = Instant::now() + Duration::from_secs(15);
        let mut seen = HashSet::new();
        while Instant::now() < deadline {
            if let Ok(paths) = rx.recv_timeout(Duration::from_millis(200)) {
                seen.extend(paths);
            }
            if matches(&seen) {
                return seen;
            }
        }
        panic!("no llegaron los cambios esperados; vistos: {seen:?}");
    }

    /// Con el sistema de archivos real: crear, editar, renombrar y borrar avisan de las
    /// rutas afectadas en segundos.
    #[test]
    fn real_changes_on_disk_are_reported() {
        let dir = tempfile::tempdir().unwrap();
        // La ruta "larga" real: en Windows, la carpeta temporal puede venir abreviada.
        let root = dir.path().canonicalize().unwrap();
        let root = PathBuf::from(root.display().to_string().trim_start_matches(r"\\?\"));
        let (tx, rx) = mpsc::channel();
        let mut watcher = FolderWatcher::start(move |paths| {
            let _ = tx.send(paths);
        })
        .unwrap();
        watcher.watch(std::slice::from_ref(&root));
        assert_eq!(watcher.watched(), std::slice::from_ref(&root));

        let a = root.join("sub").join("apuntes.md");
        std::fs::create_dir_all(a.parent().unwrap()).unwrap();
        std::fs::write(&a, "uno").unwrap();
        wait_for(&rx, |seen| seen.contains(&a));

        std::fs::write(&a, "dos, editado").unwrap();
        wait_for(&rx, |seen| seen.contains(&a));

        let b = a.with_file_name("tema4.md");
        std::fs::rename(&a, &b).unwrap();
        // Un renombrado avisa de las dos rutas: la vieja y la nueva.
        wait_for(&rx, |seen| seen.contains(&a) && seen.contains(&b));

        std::fs::remove_file(&b).unwrap();
        wait_for(&rx, |seen| seen.contains(&b));

        // Fuera de las carpetas vigiladas no se oye nada.
        watcher.watch(&[]);
        assert!(watcher.watched().is_empty());
    }
}
