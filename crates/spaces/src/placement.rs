//! Colocar ventanas en zonas de la pantalla (Fase 6.5, ADR 0014). La parte pura: las
//! zonas, la geometría y cómo se elige la ventana. Lo que toca Windows (enumerar
//! ventanas y pantallas, moverlas) está detrás del trait [`WindowManager`].

use serde::{Deserialize, Serialize};

/// Dónde se coloca una ventana al abrir una acción.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct Placement {
    pub zone: Zone,
    #[serde(default)]
    pub monitor: MonitorChoice,
    /// «Si ya está abierta, moverla en vez de abrir otra».
    #[serde(default)]
    pub reuse_window: bool,
}

/// Zonas predefinidas (las propias, al backlog).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub enum Zone {
    Maximized,
    LeftHalf,
    RightHalf,
    TopHalf,
    BottomHalf,
    LeftThird,
    CenterThird,
    RightThird,
    LeftTwoThirds,
    RightTwoThirds,
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}

impl Zone {
    pub const ALL: [Zone; 14] = [
        Zone::Maximized,
        Zone::LeftHalf,
        Zone::RightHalf,
        Zone::TopHalf,
        Zone::BottomHalf,
        Zone::LeftThird,
        Zone::CenterThird,
        Zone::RightThird,
        Zone::LeftTwoThirds,
        Zone::RightTwoThirds,
        Zone::TopLeft,
        Zone::TopRight,
        Zone::BottomLeft,
        Zone::BottomRight,
    ];

    /// La zona en fracciones de la pantalla: (izquierda, arriba, ancho, alto), en sextos
    /// (así mitades y tercios son enteros y no hay redondeos acumulados).
    fn sixths(self) -> (i32, i32, i32, i32) {
        match self {
            Zone::Maximized => (0, 0, 6, 6),
            Zone::LeftHalf => (0, 0, 3, 6),
            Zone::RightHalf => (3, 0, 3, 6),
            Zone::TopHalf => (0, 0, 6, 3),
            Zone::BottomHalf => (0, 3, 6, 3),
            Zone::LeftThird => (0, 0, 2, 6),
            Zone::CenterThird => (2, 0, 2, 6),
            Zone::RightThird => (4, 0, 2, 6),
            Zone::LeftTwoThirds => (0, 0, 4, 6),
            Zone::RightTwoThirds => (2, 0, 4, 6),
            Zone::TopLeft => (0, 0, 3, 3),
            Zone::TopRight => (3, 0, 3, 3),
            Zone::BottomLeft => (0, 3, 3, 3),
            Zone::BottomRight => (3, 3, 3, 3),
        }
    }

    /// El nombre que se ve («Mitad izquierda»).
    pub fn label(self) -> &'static str {
        match self {
            Zone::Maximized => "toda la pantalla",
            Zone::LeftHalf => "la mitad izquierda",
            Zone::RightHalf => "la mitad derecha",
            Zone::TopHalf => "la mitad de arriba",
            Zone::BottomHalf => "la mitad de abajo",
            Zone::LeftThird => "el tercio izquierdo",
            Zone::CenterThird => "el tercio central",
            Zone::RightThird => "el tercio derecho",
            Zone::LeftTwoThirds => "los dos tercios izquierdos",
            Zone::RightTwoThirds => "los dos tercios derechos",
            Zone::TopLeft => "el cuarto de arriba a la izquierda",
            Zone::TopRight => "el cuarto de arriba a la derecha",
            Zone::BottomLeft => "el cuarto de abajo a la izquierda",
            Zone::BottomRight => "el cuarto de abajo a la derecha",
        }
    }
}

/// En qué pantalla.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum MonitorChoice {
    /// La principal de Windows.
    #[default]
    Primary,
    /// Donde esté el ratón al abrir el Espacio.
    UnderMouse,
    /// Una concreta: por su nombre («DELL U2720Q») y, si hay dos con el mismo nombre o
    /// cambia, por dónde estaba (su esquina de arriba a la izquierda).
    Specific { name: String, left: i32, top: i32 },
}

/// Un rectángulo en píxeles de la pantalla (el escritorio virtual entero).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct Rect {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

impl Rect {
    pub fn width(&self) -> i32 {
        self.right - self.left
    }
    pub fn height(&self) -> i32 {
        self.bottom - self.top
    }
    pub fn contains(&self, x: i32, y: i32) -> bool {
        x >= self.left && x < self.right && y >= self.top && y < self.bottom
    }
}

/// El rectángulo de `zone` dentro del área útil de una pantalla (sin la barra de tareas).
pub fn zone_rect(zone: Zone, work: Rect) -> Rect {
    let (l, t, w, h) = zone.sixths();
    let x = |sixths: i32| work.left + work.width() * sixths / 6;
    let y = |sixths: i32| work.top + work.height() * sixths / 6;
    Rect {
        left: x(l),
        top: y(t),
        right: x(l + w),
        bottom: y(t + h),
    }
}

/// Windows 10 pone a las ventanas un borde invisible (para agarrarlas al redimensionar):
/// su rectángulo es más grande que lo que se ve. Para que lo **visible** ocupe `target`,
/// se agranda `target` en lo que mide ese borde. `window` es el rectángulo de la ventana
/// y `visible`, el de lo que se ve (`DWMWA_EXTENDED_FRAME_BOUNDS`).
pub fn with_invisible_borders(target: Rect, window: Rect, visible: Rect) -> Rect {
    Rect {
        left: target.left - (visible.left - window.left),
        top: target.top - (visible.top - window.top),
        right: target.right + (window.right - visible.right),
        bottom: target.bottom + (window.bottom - visible.bottom),
    }
}

/// Una pantalla.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct MonitorInfo {
    /// Nombre del modelo si Windows lo sabe («DELL U2720Q»); si no, «Pantalla 2».
    pub name: String,
    /// La pantalla entera y el área útil (sin la barra de tareas).
    pub bounds: Rect,
    pub work: Rect,
    pub primary: bool,
}

/// La pantalla elegida, entre las que hay ahora. Si la concreta ya no está (portátil sin
/// el monitor), la principal: mejor colocarla ahí que no colocarla.
pub fn pick_monitor<'a>(
    choice: &MonitorChoice,
    monitors: &'a [MonitorInfo],
    mouse: Option<(i32, i32)>,
) -> Option<&'a MonitorInfo> {
    let primary = monitors.iter().find(|m| m.primary).or(monitors.first());
    match choice {
        MonitorChoice::Primary => primary,
        MonitorChoice::UnderMouse => mouse
            .and_then(|(x, y)| monitors.iter().find(|m| m.bounds.contains(x, y)))
            .or(primary),
        MonitorChoice::Specific { name, left, top } => {
            let at_place = |m: &MonitorInfo| m.bounds.left == *left && m.bounds.top == *top;
            // 1. Mismo nombre y mismo sitio. 2. Mismo nombre. 3. Mismo sitio. 4. Principal.
            monitors
                .iter()
                .find(|m| m.name == *name && at_place(m))
                .or_else(|| monitors.iter().find(|m| m.name == *name))
                .or_else(|| monitors.iter().find(|m| at_place(m)))
                .or(primary)
        }
    }
}

/// Una ventana abierta, como la ve Sift.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WindowInfo {
    /// El identificador de Windows (`HWND`).
    pub id: isize,
    /// El programa, solo el nombre y en minúsculas («code.exe»).
    pub exe: String,
    pub title: String,
}

/// Qué ventana se busca tras abrir algo: de qué programa (si se sabe) y, para desempatar,
/// algo que suela salir en su título (el nombre de la carpeta, del archivo…).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct WindowTarget {
    pub exe: Option<String>,
    pub title_hint: Option<String>,
}

/// La ventana buscada entre `windows`, sin contar las de `before` (las que ya estaban
/// abiertas antes de lanzarla; vacío para buscar entre las existentes). Del programa
/// esperado, si se sabe, y entre varias, la que tiene la pista en el título.
pub fn pick_window<'a>(
    windows: &'a [WindowInfo],
    before: &[isize],
    target: &WindowTarget,
) -> Option<&'a WindowInfo> {
    let fresh: Vec<&WindowInfo> = windows
        .iter()
        .filter(|w| !before.contains(&w.id))
        .filter(|w| target.exe.as_deref().is_none_or(|exe| w.exe == exe))
        .collect();
    let hint = target.title_hint.as_deref().map(str::to_lowercase);
    hint.and_then(|h| {
        fresh
            .iter()
            .find(|w| w.title.to_lowercase().contains(&h))
            .copied()
    })
    .or_else(|| fresh.first().copied())
}

/// Lo que hace falta de Windows para colocar ventanas.
pub trait WindowManager {
    /// Las ventanas normales visibles (sin las de herramientas ni las ocultas).
    fn windows(&self) -> Vec<WindowInfo>;
    fn monitors(&self) -> Vec<MonitorInfo>;
    fn mouse(&self) -> Option<(i32, i32)>;
    /// Qué ventana buscar tras abrir esta acción.
    fn target(&self, action: &crate::model::Action) -> WindowTarget;
    /// Mueve la ventana a la zona y la trae al frente.
    fn place(&self, window: isize, zone: Zone, monitor: &MonitorInfo) -> Result<(), String>;
}

#[cfg(test)]
mod tests {
    use super::*;

    const WORK: Rect = Rect {
        left: 0,
        top: 0,
        right: 1920,
        bottom: 1040,
    };

    #[test]
    fn zones_split_the_work_area() {
        assert_eq!(
            zone_rect(Zone::RightHalf, WORK),
            Rect {
                left: 960,
                top: 0,
                right: 1920,
                bottom: 1040
            }
        );
        assert_eq!(zone_rect(Zone::CenterThird, WORK).left, 640);
        assert_eq!(zone_rect(Zone::CenterThird, WORK).right, 1280);
        assert_eq!(zone_rect(Zone::BottomLeft, WORK).top, 520);
        assert_eq!(zone_rect(Zone::Maximized, WORK), WORK);
    }

    #[test]
    fn zones_on_a_second_monitor_use_its_coordinates() {
        let right = Rect {
            left: 1920,
            top: -200,
            right: 4480,
            bottom: 1240,
        };
        let r = zone_rect(Zone::LeftHalf, right);
        assert_eq!((r.left, r.top, r.right, r.bottom), (1920, -200, 3200, 1240));
    }

    #[test]
    fn neighbouring_zones_touch_without_gaps() {
        for (a, b) in [
            (Zone::LeftThird, Zone::CenterThird),
            (Zone::CenterThird, Zone::RightThird),
            (Zone::LeftTwoThirds, Zone::RightThird),
            (Zone::LeftHalf, Zone::RightHalf),
        ] {
            assert_eq!(zone_rect(a, WORK).right, zone_rect(b, WORK).left);
        }
        // Un área que no se divide exacta (1366 entre 3).
        let odd = Rect {
            right: 1366,
            ..WORK
        };
        assert_eq!(
            zone_rect(Zone::LeftThird, odd).right,
            zone_rect(Zone::CenterThird, odd).left
        );
    }

    #[test]
    fn invisible_borders_are_added_back() {
        // Windows 10: 7 px invisibles a los lados y abajo, nada arriba.
        let window = Rect {
            left: 93,
            top: 100,
            right: 907,
            bottom: 707,
        };
        let visible = Rect {
            left: 100,
            top: 100,
            right: 900,
            bottom: 700,
        };
        let target = zone_rect(Zone::LeftHalf, WORK);
        let set = with_invisible_borders(target, window, visible);
        assert_eq!(
            (set.left, set.top, set.right, set.bottom),
            (-7, 0, 967, 1047)
        );
    }

    fn monitor(name: &str, left: i32, primary: bool) -> MonitorInfo {
        let bounds = Rect {
            left,
            top: 0,
            right: left + 1920,
            bottom: 1080,
        };
        MonitorInfo {
            name: name.into(),
            bounds,
            work: Rect {
                bottom: 1040,
                ..bounds
            },
            primary,
        }
    }

    #[test]
    fn picks_the_chosen_monitor_or_falls_back_to_the_primary() {
        let monitors = [monitor("DELL", 0, true), monitor("LG", 1920, false)];
        let pick =
            |c: &MonitorChoice, mouse| pick_monitor(c, &monitors, mouse).unwrap().name.clone();
        assert_eq!(pick(&MonitorChoice::Primary, None), "DELL");
        assert_eq!(pick(&MonitorChoice::UnderMouse, Some((2500, 300))), "LG");
        let lg = MonitorChoice::Specific {
            name: "LG".into(),
            left: 1920,
            top: 0,
        };
        assert_eq!(pick(&lg, None), "LG");
        // Sin el LG conectado: a la principal.
        let only_dell = [monitor("DELL", 0, true)];
        assert_eq!(pick_monitor(&lg, &only_dell, None).unwrap().name, "DELL");
        // Dos iguales: la que está donde estaba.
        let twins = [monitor("LG", 0, true), monitor("LG", 1920, false)];
        assert_eq!(pick_monitor(&lg, &twins, None).unwrap().bounds.left, 1920);
    }

    fn win(id: isize, exe: &str, title: &str) -> WindowInfo {
        WindowInfo {
            id,
            exe: exe.into(),
            title: title.into(),
        }
    }

    #[test]
    fn picks_the_new_window_of_the_expected_program() {
        let windows = [
            win(1, "code.exe", "Sift - Visual Studio Code"),
            win(2, "chrome.exe", "Campus"),
            win(3, "code.exe", "Apuntes - Visual Studio Code"),
        ];
        let code = WindowTarget {
            exe: Some("code.exe".into()),
            title_hint: None,
        };
        // VS Code reutiliza su proceso: la nueva es la que no estaba antes.
        assert_eq!(pick_window(&windows, &[1, 2], &code).unwrap().id, 3);
        // Ninguna nueva todavía.
        assert!(pick_window(&windows, &[1, 2, 3], &code).is_none());
        // Entre varias del mismo programa, la que tiene la pista en el título.
        let apuntes = WindowTarget {
            exe: Some("code.exe".into()),
            title_hint: Some("apuntes".into()),
        };
        assert_eq!(pick_window(&windows, &[], &apuntes).unwrap().id, 3);
        // Sin saber el programa: la primera ventana nueva.
        assert_eq!(
            pick_window(&windows, &[1], &WindowTarget::default())
                .unwrap()
                .id,
            2
        );
    }
}
