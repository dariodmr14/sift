//! Colocar ventanas sin bloquear (ADR 0014, cambio del 2026-11-19).
//!
//! Antes, cada acción que colocaba una ventana esperaba a que esa ventana apareciera (hasta
//! 15 s) antes de lanzar la siguiente: con siete apps lentas, la última tardaba muchísimo.
//! Ahora la acción se lanza, se apunta aquí qué ventana hay que colocar y se sigue. Un solo
//! bucle ([`Placer::poll`]) mira las ventanas abiertas y coloca cada una cuando aparece su
//! ventana principal, con mucho más margen ([`WINDOW_TIMEOUT`]) porque ya no retrasa nada.
//!
//! - **Ventanas intermedias** (un selector, una pantalla de carga, «actualizando…») no se
//!   colocan: se sigue esperando a la principal.
//! - **Si la ventana colocada se cierra** antes de asentarse ([`SETTLE`]) — el selector de
//!   workspace de Eclipse —, se sigue esperando a la siguiente del mismo programa.
//! - **Una ventana, una acción**: dos acciones nunca se quedan con la misma ventana.

use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::model::Action;
use crate::placement::{
    MonitorInfo, Placement, WindowInfo, WindowManager, WindowTarget, Zone, pick_monitor,
    pick_window,
};
use crate::run::Progress;

/// Cuánto se espera, como mucho, a la ventana de una app para colocarla. Como no bloquea
/// las demás acciones, puede ser largo (un IDE que arranca, alguien que elige su workspace).
pub const WINDOW_TIMEOUT: Duration = Duration::from_secs(60);
/// Cuánto tiene que seguir abierta la ventana colocada para darla por buena.
pub const SETTLE: Duration = Duration::from_millis(1500);
/// Cada cuánto se mira si han aparecido las ventanas pendientes.
pub const WINDOW_POLL: Duration = Duration::from_millis(250);
/// Una ventana sin barra de título se toma por pantalla de carga. Si sigue ahí todo este
/// rato y no sale otra mejor, es que la app es así: se coloca esa.
pub const FRAMELESS_GRACE: Duration = Duration::from_secs(20);

/// En qué va la colocación de la ventana de una acción (para el panel «en marcha»).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub enum PlacementState {
    /// Abierta; esperando a su ventana para colocarla.
    Placing,
    /// Colocada (o la ya abierta, traída al frente).
    Placed,
    /// No se pudo colocar (el motivo va en la nota de la acción).
    NotPlaced,
}

/// Palabras de los títulos de ventanas que no son la principal: selectores, cargas,
/// actualizaciones. La comparación no distingue mayúsculas.
const INTERMEDIATE: [&str; 9] = [
    "launcher",
    "loading",
    "cargando",
    "splash",
    "starting",
    "iniciando",
    "updating",
    "updater",
    "actualizando",
];

/// ¿Es una ventana intermedia (no la principal del programa)? Por su título: «Eclipse IDE
/// Launcher», «Loading…», «Actualizando…».
pub fn is_intermediate(title: &str) -> bool {
    let t = title.to_lowercase();
    INTERMEDIATE.iter().any(|w| t.contains(w))
}

/// ¿Puede ser la ventana principal ya? No si su título es de una intermedia ni si no
/// tiene barra de título (la pantalla de carga de Eclipse, un `Static` de 452 × 302).
fn looks_main(w: &WindowInfo) -> bool {
    !w.frameless && !is_intermediate(&w.title)
}

struct Pending {
    index: u32,
    target: WindowTarget,
    zone: Zone,
    place: Placement,
    /// Las ventanas que ya había justo antes de abrirla.
    before: Vec<isize>,
    started: Duration,
    /// La ventana colocada y cuándo, mientras no se ha asentado.
    placed: Option<(isize, Duration)>,
    /// La última ventana intermedia vista, para decirlo si no aparece la principal.
    intermediate: Option<String>,
    /// Una ventana sin barra de título que podría ser la principal, y desde cuándo.
    frameless: Option<(isize, Duration)>,
    note: Option<String>,
    done: bool,
}

/// Coloca las ventanas de las acciones según van apareciendo, sin bloquear.
pub struct Placer<'a> {
    wm: &'a dyn WindowManager,
    pending: Vec<Pending>,
    /// Ventanas que ya son de alguna acción.
    claimed: Vec<isize>,
}

fn put(
    wm: &dyn WindowManager,
    window: isize,
    zone: Zone,
    place: &Placement,
    monitors: &[MonitorInfo],
) -> Option<String> {
    let Some(monitor) = pick_monitor(&place.monitor, monitors, wm.mouse()) else {
        return Some("no encontré ninguna pantalla para colocarla".into());
    };
    wm.place(window, zone, monitor)
        .err()
        .map(|e| format!("se abrió, pero no se pudo colocar: {e}"))
}

impl<'a> Placer<'a> {
    pub fn new(wm: &'a dyn WindowManager) -> Self {
        Placer {
            wm,
            pending: Vec::new(),
            claimed: Vec::new(),
        }
    }

    /// «Si ya está abierta, usar esa»: si hay una ventana de la app que no sea ya de otra
    /// acción, se coloca (o, sin zona, se trae al frente) y no hace falta abrir nada.
    /// `None` si no la hay; `Some(nota)` si se usó (la nota, si algo salió mal).
    pub fn reuse(
        &mut self,
        index: u32,
        action: &Action,
        place: &Placement,
        progress: &dyn Progress,
    ) -> Option<Option<String>> {
        let target = self.wm.target(action);
        let free: Vec<WindowInfo> = self
            .wm
            .windows()
            .into_iter()
            .filter(|w| !self.claimed.contains(&w.id) && looks_main(w))
            .collect();
        let existing = pick_window(&free, &[], &target)?.id;
        self.claimed.push(existing);
        let note = match place.zone {
            None => self
                .wm
                .focus(existing)
                .err()
                .map(|e| format!("ya estaba abierta, pero no se pudo traer al frente: {e}")),
            Some(zone) => put(self.wm, existing, zone, place, &self.wm.monitors()),
        };
        let state = if note.is_none() {
            PlacementState::Placed
        } else {
            PlacementState::NotPlaced
        };
        progress.placement(index, state);
        Some(note)
    }

    /// Las ventanas abiertas ahora: lo que había antes de abrir algo.
    pub fn snapshot(&self) -> Vec<isize> {
        self.wm.windows().iter().map(|w| w.id).collect()
    }

    /// Se acaba de abrir `action`: hay que colocar su ventana cuando aparezca.
    pub fn track(
        &mut self,
        index: u32,
        action: &Action,
        place: &Placement,
        before: Vec<isize>,
        now: Duration,
        progress: &dyn Progress,
    ) {
        let Some(zone) = place.zone else {
            return;
        };
        self.pending.push(Pending {
            index,
            target: self.wm.target(action),
            zone,
            place: place.clone(),
            before,
            started: now,
            placed: None,
            intermediate: None,
            frameless: None,
            note: None,
            done: false,
        });
        progress.placement(index, PlacementState::Placing);
    }

    /// ¿Queda alguna ventana por colocar?
    pub fn busy(&self) -> bool {
        self.pending.iter().any(|p| !p.done)
    }

    /// Una vuelta: mira las ventanas abiertas y coloca las que hayan aparecido.
    pub fn poll(&mut self, now: Duration, progress: &dyn Progress) {
        if !self.busy() {
            return;
        }
        let windows = self.wm.windows();
        let monitors = self.wm.monitors();
        let Placer {
            wm,
            pending,
            claimed,
        } = self;
        for p in pending.iter_mut().filter(|p| !p.done) {
            if let Some((id, at)) = p.placed {
                if windows.iter().any(|w| w.id == id) {
                    if now.saturating_sub(at) >= SETTLE {
                        p.done = true;
                        progress.placement(p.index, PlacementState::Placed);
                    }
                    continue;
                }
                // Se cerró antes de asentarse (un selector): a por la siguiente.
                claimed.retain(|c| *c != id);
                p.placed = None;
            }
            let (main, intermediate): (Vec<WindowInfo>, Vec<WindowInfo>) = windows
                .iter()
                .filter(|w| !claimed.contains(&w.id))
                .cloned()
                .partition(looks_main);
            let mut chosen = pick_window(&main, &p.before, &p.target).map(|w| w.id);
            if chosen.is_none()
                && let Some(w) = pick_window(&intermediate, &p.before, &p.target)
            {
                p.intermediate = Some(w.title.clone());
                // Sin barra de título pero con un título normal: si dura, es la principal.
                if w.frameless && !is_intermediate(&w.title) {
                    match p.frameless {
                        Some((id, since)) if id == w.id => {
                            if now.saturating_sub(since) >= FRAMELESS_GRACE {
                                chosen = Some(id);
                            }
                        }
                        _ => p.frameless = Some((w.id, now)),
                    }
                }
            }
            if let Some(id) = chosen {
                match put(*wm, id, p.zone, &p.place, &monitors) {
                    None => {
                        claimed.push(id);
                        p.placed = Some((id, now));
                    }
                    Some(error) => {
                        p.note = Some(error);
                        p.done = true;
                        progress.placement(p.index, PlacementState::NotPlaced);
                    }
                }
                continue;
            }
            if now.saturating_sub(p.started) >= WINDOW_TIMEOUT {
                p.done = true;
                p.note = Some(match &p.intermediate {
                    Some(title) => {
                        format!("se abrió, pero solo vi su ventana «{title}»: no la coloqué")
                    }
                    None => "se abrió, pero no encontré su ventana para colocarla".into(),
                });
                progress.placement(p.index, PlacementState::NotPlaced);
            }
        }
    }

    /// Cómo quedó cada acción que había que colocar: `(índice, nota)`. Lo que quedaba por
    /// colocar al cancelar se dice.
    pub fn finish(self, cancelled: bool) -> Vec<(u32, Option<String>)> {
        self.pending
            .into_iter()
            .map(|p| {
                let note = if p.done || p.placed.is_some() {
                    p.note
                } else if cancelled {
                    Some("cancelado antes de colocar su ventana".into())
                } else {
                    Some("se abrió, pero no encontré su ventana para colocarla".into())
                };
                (p.index, note)
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn intermediate_windows_are_recognised_by_their_title() {
        assert!(is_intermediate("Eclipse IDE Launcher"));
        assert!(is_intermediate("Cargando…"));
        assert!(is_intermediate("Discord Updater"));
        assert!(!is_intermediate("ws - Main.java - Eclipse IDE"));
        assert!(!is_intermediate("Acceso rápido - Opera"));
    }
}
