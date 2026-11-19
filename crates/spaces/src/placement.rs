//! Colocar ventanas en zonas de la pantalla (Fase 6.5, ADR 0014). La parte pura: las
//! zonas, la geometría y cómo se elige la ventana. Lo que toca Windows (enumerar
//! ventanas y pantallas, moverlas) está detrás del trait [`WindowManager`].

use serde::{Deserialize, Serialize};

/// Dónde se coloca una ventana al abrir una acción.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct Placement {
    /// La zona. `None` = no moverla (donde la ponga Windows): sirve para «si ya está
    /// abierta, usar esa» sin colocarla. Los Espacios guardados antes siempre la tienen.
    #[serde(default)]
    pub zone: Option<Zone>,
    #[serde(default)]
    pub monitor: MonitorChoice,
    /// «Si ya está abierta, usar esa en vez de abrir otra» (y colocarla, si hay zona).
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
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct WindowInfo {
    /// El identificador de Windows (`HWND`).
    pub id: isize,
    /// El programa, solo el nombre y en minúsculas («code.exe»).
    pub exe: String,
    pub title: String,
    /// La ruta completa del programa, en minúsculas (vacía si Windows no la da).
    pub path: String,
    /// Los programas de los procesos padre, del más cercano al más lejano, en minúsculas.
    /// Eclipse, por ejemplo: sus ventanas son de `javaw.exe`, nieto de `eclipse.exe`.
    pub ancestors: Vec<String>,
    /// El id de app de Windows (AUMID) de la ventana o de su paquete, si lo tiene: las apps
    /// de la Tienda (Claude) lo llevan en su proceso.
    pub app_id: Option<String>,
    /// Ancho y alto en píxeles (0 si no se sabe).
    pub size: (i32, i32),
}

/// Qué ventana se busca tras abrir algo: de qué programa y, para desempatar, algo que suela
/// salir en su título (el nombre de la carpeta, del archivo…). Cuanto más se sepa del
/// programa, menos se depende del título.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct WindowTarget {
    /// El programa, solo el nombre y en minúsculas («code.exe»).
    pub exe: Option<String>,
    /// La carpeta de instalación, en minúsculas y acabada en `\`: vale cualquier programa de
    /// dentro. Opera abre `launcher.exe`, pero sus ventanas son de `opera.exe`.
    pub folder: Option<String>,
    /// El id de app (AUMID), para las apps de la Tienda.
    pub app_id: Option<String>,
    pub title_hint: Option<String>,
}

impl WindowTarget {
    /// ¿Se sabe algo del programa, aparte del título?
    pub fn knows_program(&self) -> bool {
        self.exe.is_some() || self.folder.is_some() || self.app_id.is_some()
    }
}

/// Lo más pequeño que puede ser la ventana principal de un programa: lo que no llega son
/// pantallas de carga, avisos o selectores.
pub const MIN_WINDOW: (i32, i32) = (320, 200);

/// ¿Es `w` del programa que se busca? Por su id de app, por el programa (o el de un proceso
/// padre) o por estar instalado en su carpeta. Sin nada de eso, nunca.
pub fn belongs(w: &WindowInfo, target: &WindowTarget) -> bool {
    let same_app = target.app_id.as_deref().is_some_and(|id| {
        w.app_id
            .as_deref()
            .is_some_and(|own| own.eq_ignore_ascii_case(id))
    });
    let same_exe = target
        .exe
        .as_deref()
        .is_some_and(|exe| w.exe == exe || w.ancestors.iter().any(|a| a == exe));
    let same_folder = target
        .folder
        .as_deref()
        .is_some_and(|folder| !w.path.is_empty() && w.path.starts_with(folder));
    same_app || same_exe || same_folder
}

fn big_enough(w: &WindowInfo) -> bool {
    let (width, height) = w.size;
    // 0: tamaño desconocido; no se descarta por eso.
    width == 0 || (width >= MIN_WINDOW.0 && height >= MIN_WINDOW.1)
}

/// La ventana buscada entre `windows`, sin contar las de `before` (las que ya estaban
/// abiertas antes de lanzarla; vacío para buscar entre las existentes). Del programa
/// esperado ([`belongs`]) y de tamaño de ventana principal; entre varias, la que tiene la
/// pista en el título.
///
/// Si no se sabe nada del programa, solo vale una ventana cuyo título lo nombre. Nunca «la
/// primera que haya»: así una acción se quedaba con la ventana de otra (FEEDBACK
/// 2026-11-19: la de Opera movió la de Claude).
pub fn pick_window<'a>(
    windows: &'a [WindowInfo],
    before: &[isize],
    target: &WindowTarget,
) -> Option<&'a WindowInfo> {
    let hint = target.title_hint.as_deref().map(str::to_lowercase);
    let titled = |w: &WindowInfo| {
        hint.as_deref()
            .is_some_and(|h| w.title.to_lowercase().contains(h))
    };
    let candidates: Vec<&WindowInfo> = windows
        .iter()
        .filter(|w| !before.contains(&w.id) && big_enough(w))
        .filter(|w| {
            if target.knows_program() {
                belongs(w, target)
            } else {
                titled(w)
            }
        })
        .collect();
    candidates
        .iter()
        .find(|w| titled(w))
        .or_else(|| candidates.first())
        .copied()
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
    /// Trae al frente una ventana sin moverla (y la restaura si estaba minimizada).
    fn focus(&self, window: isize) -> Result<(), String>;
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
            size: (1200, 800),
            ..WindowInfo::default()
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
            ..WindowTarget::default()
        };
        // VS Code reutiliza su proceso: la nueva es la que no estaba antes.
        assert_eq!(pick_window(&windows, &[1, 2], &code).unwrap().id, 3);
        // Ninguna nueva todavía.
        assert!(pick_window(&windows, &[1, 2, 3], &code).is_none());
        // Entre varias del mismo programa, la que tiene la pista en el título.
        let apuntes = WindowTarget {
            exe: Some("code.exe".into()),
            title_hint: Some("apuntes".into()),
            ..WindowTarget::default()
        };
        assert_eq!(pick_window(&windows, &[], &apuntes).unwrap().id, 3);
        // Sin saber nada del programa, ni la pista en ningún título: ninguna.
        assert!(pick_window(&windows, &[1], &WindowTarget::default()).is_none());
    }

    /// El caso de Darío (FEEDBACK 2026-11-19): Claude (de la Tienda) y Opera (que abre
    /// `launcher.exe` y cuyas ventanas son de `opera.exe`), con «usar la ya abierta». Una
    /// pestaña de Opera se llama «Claude - Opera».
    #[test]
    fn each_app_keeps_its_own_window_even_with_misleading_titles() {
        let claude = WindowInfo {
            app_id: Some("Claude_pzs8sxrjxfjjc!Claude".into()),
            path: r"c:\program files\windowsapps\claude_1.0_x64__pzs8sxrjxfjjc\app\claude.exe"
                .into(),
            ..win(10, "claude.exe", "Claude")
        };
        let opera_tab = WindowInfo {
            path: r"c:\users\ana\appdata\local\programs\opera gx\124.0\opera.exe".into(),
            ..win(20, "opera.exe", "Claude - Opera")
        };
        let windows = [opera_tab, claude.clone()];
        let claude_target = WindowTarget {
            app_id: Some("claude_pzs8sxrjxfjjc!claude".into()),
            title_hint: Some("Claude".into()),
            ..WindowTarget::default()
        };
        let opera_target = WindowTarget {
            exe: Some("launcher.exe".into()),
            folder: Some(r"c:\users\ana\appdata\local\programs\opera gx\".into()),
            app_id: Some("OperaSoftware.OperaGXWebBrowser.1759849920".into()),
            title_hint: Some("Navegador Opera GX".into()),
        };
        // Claude, por su id de app, aunque la pestaña de Opera tenga «Claude» en el título.
        assert_eq!(pick_window(&windows, &[], &claude_target).unwrap().id, 10);
        // Opera, por su carpeta, aunque el título no diga «Navegador Opera GX».
        assert_eq!(pick_window(&windows, &[], &opera_target).unwrap().id, 20);
        // Sin ventanas de Opera abiertas, la acción de Opera no se queda con la de Claude.
        assert!(pick_window(&[claude], &[], &opera_target).is_none());
    }

    /// Eclipse: sus ventanas son de `javaw.exe`, nieto de `eclipse.exe`. Las pequeñas (la
    /// pantalla de carga) no cuentan.
    #[test]
    fn a_window_of_a_child_process_counts_and_small_ones_do_not() {
        let eclipse = WindowTarget {
            exe: Some("eclipse.exe".into()),
            title_hint: Some("Eclipse IDE".into()),
            ..WindowTarget::default()
        };
        let family = vec!["javaw.exe".to_string(), "eclipse.exe".to_string()];
        let tiny = WindowInfo {
            ancestors: family.clone(),
            size: (200, 120),
            ..win(1, "javaw.exe", "")
        };
        let launcher = WindowInfo {
            ancestors: family.clone(),
            size: (700, 450),
            ..win(3, "javaw.exe", "Eclipse IDE Launcher")
        };
        let main = WindowInfo {
            ancestors: family,
            ..win(2, "javaw.exe", "ws - Main.java - Eclipse IDE")
        };
        assert_eq!(
            pick_window(std::slice::from_ref(&tiny), &[], &eclipse),
            None
        );
        assert_eq!(pick_window(&[tiny, main], &[], &eclipse).unwrap().id, 2);
        // El selector de workspace cuenta como ventana de Eclipse: de esperar a la principal
        // cuando se cierra se encarga quien coloca (ver `run.rs`).
        assert_eq!(pick_window(&[launcher], &[], &eclipse).unwrap().id, 3);
    }
}
