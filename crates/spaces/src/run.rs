//! Ejecutar las acciones de un Espacio.
//!
//! La lógica (qué se lanza, en qué orden, qué hay que confirmar, cuánto se espera) está
//! separada de *cómo* se hace en Windows mediante dos traits: [`Opener`] (abrir y lanzar)
//! y [`Waiter`] (comprobar si algo está listo y dejar pasar el tiempo). La app usa los de
//! verdad; los tests, unos falsos que apuntan lo que se les pide y no esperan de verdad.
//!
//! Esperas (ADR 0012): un paso «Esperar a que…» comprueba su condición cada medio
//! segundo hasta que se cumple o se acaba su tiempo. Si no se cumple, según el paso, se
//! para (lo que queda no se ejecuta) o se sigue. Cancelar también para. **Nunca se
//! cierra lo que ya se abrió o lanzó.**

use std::collections::HashSet;
use std::path::Path;
use std::time::Duration;

use serde::Serialize;

use crate::model::{Action, CommandSpec, OnTimeout, WaitSpec, WaitUntil};
use crate::placement::{Placement, WindowManager, pick_monitor, pick_window};
use crate::validate::validate_action;

/// Lo que sabe hacer el sistema operativo. Cada método lanza una sola cosa.
pub trait Opener {
    fn open_path(&self, path: &Path) -> Result<(), String>;
    fn open_url(&self, url: &str) -> Result<(), String>;
    fn open_app(&self, app_id: &str) -> Result<(), String>;
    /// Lanza `program` con `args` **como lista**, sin shell, en su propia consola.
    fn spawn(&self, command: &CommandSpec) -> Result<(), String>;
    /// Como `spawn`, pero sin ventana y guardando su salida, para poder esperar a que
    /// termine y, si falla, enseñar sus últimas líneas.
    fn spawn_captured(&self, command: &CommandSpec) -> Result<Box<dyn Child>, String>;
}

/// Un comando lanzado con [`Opener::spawn_captured`].
pub trait Child {
    /// `None` si sigue en marcha; `Some(Ok)` si terminó bien; `Some(Err(motivo))` si
    /// terminó con error (con sus últimas líneas de salida).
    fn try_finished(&mut self) -> Option<Result<(), String>>;
}

/// Comprobar si algo está listo y dejar pasar el tiempo.
pub trait Waiter {
    /// ¿Responde esta web de tu equipo? (una comprobación, rápida)
    fn web_responds(&self, url: &str) -> bool;
    /// ¿Acepta conexiones un servicio de tu equipo en este puerto?
    fn accepts_connections(&self, port: u16) -> bool;
    fn sleep(&self, d: Duration);
    /// Tiempo transcurrido desde un instante fijo (un reloj que solo avanza).
    fn now(&self) -> Duration;
}

/// Lo que la ejecución cuenta a la UI y cómo se cancela.
pub trait Progress {
    /// Empieza el paso `index` (desde 1). `waited` = segundos esperando, si es una espera.
    fn step(&self, index: u32, label: &str, waited: Option<u32>);
    fn cancelled(&self) -> bool;
}

/// Sin UI ni cancelación.
pub struct NoProgress;

impl Progress for NoProgress {
    fn step(&self, _: u32, _: &str, _: Option<u32>) {}
    fn cancelled(&self) -> bool {
        false
    }
}

/// Cada cuánto se vuelve a comprobar una espera.
pub const POLL: Duration = Duration::from_millis(500);

/// Resultado de una acción, para mostrarlo en la UI.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct ActionOutcome {
    /// Posición de la acción (desde 1).
    pub index: u32,
    pub label: String,
    /// `None` si fue bien; si no, el motivo.
    pub error: Option<String>,
    /// `false` si no llegó a ejecutarse (se paró antes o se canceló).
    pub ran: bool,
    /// Un aviso que no es un fallo: se abrió, pero algo no salió del todo («no encontré
    /// su ventana para colocarla»).
    pub note: Option<String>,
}

/// Cuánto se espera a que aparezca la ventana de lo que se abrió, para colocarla.
pub const WINDOW_TIMEOUT: Duration = Duration::from_secs(15);
/// Cada cuánto se mira si ya ha aparecido.
const WINDOW_POLL: Duration = Duration::from_millis(250);

/// Dónde colocar lo que abre una acción, si se ha pedido.
pub fn placement_of(action: &Action) -> Option<&Placement> {
    match action {
        Action::OpenFolder { place, .. }
        | Action::OpenFile { place, .. }
        | Action::OpenUrl { place, .. }
        | Action::OpenApp { place, .. } => place.as_ref(),
        _ => None,
    }
}

/// Comandos de `actions` que aún no se han aprobado, sin repetidos y en orden.
pub fn pending_approvals<'a>(
    actions: impl IntoIterator<Item = &'a Action>,
    is_approved: impl Fn(&CommandSpec) -> bool,
) -> Vec<CommandSpec> {
    let mut seen = HashSet::new();
    actions
        .into_iter()
        .filter_map(|a| match a {
            Action::RunCommand(c) if !is_approved(c) && seen.insert(c.clone()) => Some(c.clone()),
            _ => None,
        })
        .collect()
}

/// Cómo acabó una espera que no se cumplió.
enum WaitFailed {
    /// No ocurrió a tiempo (o el comando falló): el motivo.
    Failed(String),
    Cancelled,
}

/// Ejecuta las acciones en orden. Si una acción normal falla, sigue con las demás: que
/// no abra la web no es motivo para no abrir la carpeta. Una espera que no se cumple
/// para o no, según diga el paso. Cancelar para. Devuelve qué pasó con cada acción,
/// también con las que no llegaron a ejecutarse.
///
/// Antes de llamar a esto, la app comprueba con [`pending_approvals`] que no quede
/// ningún comando sin confirmar.
pub fn run_actions(
    actions: &[Action],
    opener: &dyn Opener,
    waiter: &dyn Waiter,
    windows: Option<&dyn WindowManager>,
    progress: &dyn Progress,
) -> Vec<ActionOutcome> {
    let mut out = Vec::with_capacity(actions.len());
    // El último comando lanzado para esperarlo (solo si el paso siguiente lo espera).
    let mut child: Option<Box<dyn Child>> = None;
    let mut stop_at: Option<usize> = None;
    for (i, action) in actions.iter().enumerate() {
        let index = i as u32 + 1;
        let label = label(action);
        if progress.cancelled() {
            stop_at = Some(i);
            break;
        }
        progress.step(index, &label, None);
        let mut note = None;
        let (error, ran, stop) = match action {
            Action::Wait(spec) => match wait(spec, child.take(), waiter, progress, index, &label) {
                Ok(()) => (None, true, false),
                Err(WaitFailed::Cancelled) => {
                    (Some("cancelado mientras esperaba".to_owned()), false, true)
                }
                Err(WaitFailed::Failed(reason)) => {
                    (Some(reason), true, spec.on_timeout == OnTimeout::Stop)
                }
            },
            Action::RunCommand(cmd) if next_waits_for_command(actions, i) => {
                let launched = check_command(cmd).and_then(|()| opener.spawn_captured(cmd));
                match launched {
                    Ok(c) => {
                        child = Some(c);
                        (None, true, false)
                    }
                    Err(e) => (Some(e), true, false),
                }
            }
            _ => match (placement_of(action), windows) {
                (Some(place), Some(wm)) => {
                    let placed =
                        open_and_place(action, place, opener, wm, waiter, progress, index, &label);
                    match placed {
                        Ok(n) => {
                            note = n;
                            (None, true, false)
                        }
                        Err(e) => (Some(e), true, false),
                    }
                }
                _ => (run_action(action, opener).err(), true, false),
            },
        };
        // Cancelar mientras se buscaba la ventana: lo abierto sigue, lo demás no se hace.
        let stop = stop || progress.cancelled();
        out.push(ActionOutcome {
            index,
            label,
            error,
            ran,
            note,
        });
        if stop {
            stop_at = Some(i + 1);
            break;
        }
    }
    // Lo que no llegó a ejecutarse, para que la UI diga qué se hizo y qué no.
    if let Some(from) = stop_at {
        for (i, action) in actions.iter().enumerate().skip(from) {
            out.push(ActionOutcome {
                index: i as u32 + 1,
                label: label(action),
                error: None,
                ran: false,
                note: None,
            });
        }
    }
    out
}

/// Abre `action` y coloca su ventana. `Ok(aviso)` si se abrió (el aviso, si no se pudo
/// colocar); `Err` si no se pudo abrir.
#[allow(clippy::too_many_arguments)]
fn open_and_place(
    action: &Action,
    place: &Placement,
    opener: &dyn Opener,
    wm: &dyn WindowManager,
    waiter: &dyn Waiter,
    progress: &dyn Progress,
    index: u32,
    label: &str,
) -> Result<Option<String>, String> {
    let target = wm.target(action);
    // Sin zona solo se reutiliza la ventana abierta: se trae al frente sin moverla y, si no
    // hay ninguna, se abre como siempre, sin esperar a su ventana.
    let Some(zone) = place.zone else {
        if place.reuse_window
            && let Some(existing) = pick_window(&wm.windows(), &[], &target)
        {
            return Ok(wm
                .focus(existing.id)
                .err()
                .map(|e| format!("ya estaba abierta, pero no se pudo traer al frente: {e}")));
        }
        return run_action(action, opener).map(|()| None);
    };
    let put = |window: isize| -> Option<String> {
        let monitors = wm.monitors();
        let Some(monitor) = pick_monitor(&place.monitor, &monitors, wm.mouse()) else {
            return Some("no encontré ninguna pantalla para colocarla".into());
        };
        wm.place(window, zone, monitor)
            .err()
            .map(|e| format!("se abrió, pero no se pudo colocar: {e}"))
    };
    // «Si ya está abierta, usar esa en vez de abrir otra».
    if place.reuse_window
        && let Some(existing) = pick_window(&wm.windows(), &[], &target)
    {
        return Ok(put(existing.id));
    }
    let before: Vec<isize> = wm.windows().iter().map(|w| w.id).collect();
    run_action(action, opener)?;
    let started = waiter.now();
    let mut reported = None;
    loop {
        if let Some(window) = pick_window(&wm.windows(), &before, &target) {
            return Ok(put(window.id));
        }
        let waited = waiter.now().saturating_sub(started);
        if waited >= WINDOW_TIMEOUT {
            return Ok(Some(
                "se abrió, pero no encontré su ventana para colocarla".into(),
            ));
        }
        if progress.cancelled() {
            return Ok(Some("cancelado antes de colocar su ventana".into()));
        }
        let secs = waited.as_secs() as u32;
        if reported != Some(secs) {
            progress.step(index, label, Some(secs));
            reported = Some(secs);
        }
        waiter.sleep(WINDOW_POLL);
    }
}

fn next_waits_for_command(actions: &[Action], i: usize) -> bool {
    matches!(
        actions.get(i + 1),
        Some(Action::Wait(WaitSpec {
            until: WaitUntil::CommandFinished,
            ..
        }))
    )
}

/// Lo que se comprueba antes de lanzar un comando: que sea válido y que exista su
/// carpeta de trabajo.
fn check_command(cmd: &CommandSpec) -> Result<(), String> {
    let mut checked = Action::RunCommand(cmd.clone());
    validate_action(&mut checked)?;
    if let Some(cwd) = &cmd.cwd
        && !cwd.is_dir()
    {
        return Err(format!(
            "no existe la carpeta de trabajo: {}",
            cwd.display()
        ));
    }
    Ok(())
}

/// Espera a que se cumpla `spec`, comprobando cada [`POLL`].
fn wait(
    spec: &WaitSpec,
    mut child: Option<Box<dyn Child>>,
    waiter: &dyn Waiter,
    progress: &dyn Progress,
    index: u32,
    label: &str,
) -> Result<(), WaitFailed> {
    let mut checked = Action::Wait(spec.clone());
    validate_action(&mut checked).map_err(WaitFailed::Failed)?;
    let Action::Wait(spec) = checked else {
        return Ok(());
    };
    let started = waiter.now();
    let limit = Duration::from_secs(u64::from(match spec.until {
        WaitUntil::Seconds { secs } => secs,
        _ => spec.timeout_secs,
    }));
    let mut reported = None;
    loop {
        let elapsed = waiter.now().saturating_sub(started);
        let secs = elapsed.as_secs() as u32;
        if reported != Some(secs) {
            progress.step(index, label, Some(secs));
            reported = Some(secs);
        }
        let ready = match &spec.until {
            WaitUntil::WebResponds { url } => waiter.web_responds(url),
            WaitUntil::ServiceAccepts { port } => waiter.accepts_connections(*port),
            WaitUntil::Seconds { .. } => elapsed >= limit,
            WaitUntil::CommandFinished => match child.as_mut() {
                None => {
                    return Err(WaitFailed::Failed(
                        "el comando de antes no se pudo lanzar".into(),
                    ));
                }
                Some(c) => match c.try_finished() {
                    None => false,
                    Some(Ok(())) => true,
                    Some(Err(reason)) => return Err(WaitFailed::Failed(reason)),
                },
            },
        };
        if ready {
            return Ok(());
        }
        if elapsed >= limit {
            return Err(WaitFailed::Failed(timeout_message(&spec)));
        }
        if progress.cancelled() {
            return Err(WaitFailed::Cancelled);
        }
        waiter.sleep(POLL);
    }
}

fn timeout_message(spec: &WaitSpec) -> String {
    let t = spec.timeout_secs;
    match &spec.until {
        WaitUntil::WebResponds { url } => format!("{url} no respondió en {t} s"),
        WaitUntil::ServiceAccepts { port } => {
            format!("nada aceptó conexiones en el puerto {port} en {t} s")
        }
        WaitUntil::CommandFinished => {
            format!("el comando no terminó en {t} s (sigue abierto)")
        }
        WaitUntil::Seconds { .. } => String::new(),
    }
}

/// Comprueba una espera **una vez**, ahora (el botón «Probar» del editor): `Ok` con lo
/// que se ha visto, o `Err` con el motivo.
pub fn check_wait_now(spec: &WaitSpec, waiter: &dyn Waiter) -> Result<String, String> {
    let mut checked = Action::Wait(spec.clone());
    validate_action(&mut checked)?;
    match &spec.until {
        WaitUntil::WebResponds { url } => {
            if waiter.web_responds(url) {
                Ok(format!("{url} responde ahora mismo."))
            } else {
                Err(format!("{url} no responde ahora mismo."))
            }
        }
        WaitUntil::ServiceAccepts { port } => {
            if waiter.accepts_connections(*port) {
                Ok(format!(
                    "Hay un servicio aceptando conexiones en el puerto {port}."
                ))
            } else {
                Err(format!(
                    "Ahora mismo nada acepta conexiones en el puerto {port}."
                ))
            }
        }
        WaitUntil::CommandFinished | WaitUntil::Seconds { .. } => {
            Err("esta espera no tiene nada que comprobar por adelantado".into())
        }
    }
}

/// Ejecuta una acción. Se vuelve a validar aquí, justo antes de lanzarla, aunque ya se
/// validó al guardar: es la última puerta antes del sistema operativo.
pub fn run_action(action: &Action, opener: &dyn Opener) -> Result<(), String> {
    let mut checked = action.clone();
    validate_action(&mut checked)?;
    match &checked {
        Action::OpenFolder { path, .. } | Action::OpenFile { path, .. } => {
            if !path.exists() {
                return Err(format!("no existe: {}", path.display()));
            }
            opener.open_path(path)
        }
        Action::OpenUrl { url, .. } => opener.open_url(url),
        Action::OpenApp { app_id, .. } => opener.open_app(app_id),
        Action::RunCommand(cmd) => {
            check_command(cmd)?;
            opener.spawn(cmd)
        }
        // Una espera suelta no lanza nada (se ejecuta dentro de `run_actions`).
        Action::Wait(_) => Ok(()),
    }
}

/// Texto corto que describe una acción ("Abrir carpeta Bases de Datos").
pub fn label(action: &Action) -> String {
    let file_name = |p: &Path| {
        p.file_name().map_or_else(
            || p.display().to_string(),
            |n| n.to_string_lossy().into_owned(),
        )
    };
    match action {
        Action::OpenFolder { path, .. } => format!("Abrir carpeta {}", file_name(path)),
        Action::OpenFile { path, .. } => format!("Abrir {}", file_name(path)),
        Action::OpenUrl { url, .. } => format!("Abrir {url}"),
        Action::OpenApp { name, .. } => format!("Abrir {name}"),
        Action::RunCommand(c) => format!("Ejecutar {}", display_command(c)),
        Action::Wait(w) => match &w.until {
            WaitUntil::WebResponds { url } => format!("Esperar a que responda {url}"),
            WaitUntil::ServiceAccepts { port } => {
                format!("Esperar a que el servicio del puerto {port} acepte conexiones")
            }
            WaitUntil::CommandFinished => "Esperar a que termine el comando".into(),
            WaitUntil::Seconds { secs } => format!("Esperar {secs} s"),
        },
    }
}

/// El texto del diálogo de Windows que pide confirmar comandos nuevos. La confirmación va
/// en un diálogo nativo (fuera del WebView) para que nada que se ejecute dentro de la
/// interfaz pueda darla por buena (auditoría, A4).
pub fn approval_message(owner: &str, commands: &[CommandSpec]) -> String {
    let what = if commands.len() == 1 {
        "un comando"
    } else {
        "estos comandos"
    };
    let mut text = format!("«{owner}» va a ejecutar {what} por primera vez:\n");
    for c in commands {
        text.push_str(&format!("\n    {}", display_command(c)));
        if let Some(cwd) = &c.cwd {
            text.push_str(&format!("\n        en {}", cwd.display()));
        }
    }
    text.push_str(
        "\n\nRevisa que sea lo que esperas. Solo se pregunta una vez por comando: si cambia, \
         se vuelve a preguntar.",
    );
    text
}

/// Cómo se muestra un comando en la confirmación: cada argumento por separado y
/// entrecomillado si tiene espacios, para que se vea exactamente qué se ejecutará.
pub fn display_command(c: &CommandSpec) -> String {
    let quote = |s: &str| {
        if s.is_empty() || s.contains(char::is_whitespace) {
            format!("\"{s}\"")
        } else {
            s.to_owned()
        }
    };
    std::iter::once(quote(&c.program))
        .chain(c.args.iter().map(|a| quote(a)))
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{OnTimeout, WaitSpec, WaitUntil};
    use std::cell::{Cell, RefCell};
    use std::path::PathBuf;

    /// Opener falso: apunta lo que se le pide en vez de lanzarlo.
    #[derive(Default)]
    struct Recorder {
        calls: RefCell<Vec<String>>,
        spawned: RefCell<Vec<CommandSpec>>,
        fail_urls: bool,
        /// Para `spawn_captured`: comprobaciones hasta que termina, y si falla.
        finishes_after: u32,
        command_fails: bool,
    }

    impl Opener for Recorder {
        fn open_path(&self, path: &Path) -> Result<(), String> {
            self.calls
                .borrow_mut()
                .push(format!("path:{}", path.display()));
            Ok(())
        }
        fn open_url(&self, url: &str) -> Result<(), String> {
            self.calls.borrow_mut().push(format!("url:{url}"));
            if self.fail_urls {
                Err("sin navegador".into())
            } else {
                Ok(())
            }
        }
        fn open_app(&self, app_id: &str) -> Result<(), String> {
            self.calls.borrow_mut().push(format!("app:{app_id}"));
            Ok(())
        }
        fn spawn(&self, command: &CommandSpec) -> Result<(), String> {
            self.calls
                .borrow_mut()
                .push(format!("spawn:{}", command.program));
            self.spawned.borrow_mut().push(command.clone());
            Ok(())
        }
        fn spawn_captured(&self, command: &CommandSpec) -> Result<Box<dyn Child>, String> {
            self.calls
                .borrow_mut()
                .push(format!("captured:{}", command.program));
            Ok(Box::new(FakeChild {
                polls_left: self.finishes_after,
                fails: self.command_fails,
            }))
        }
    }

    /// Un comando que termina tras `polls_left` comprobaciones (bien o mal).
    struct FakeChild {
        polls_left: u32,
        fails: bool,
    }

    impl Child for FakeChild {
        fn try_finished(&mut self) -> Option<Result<(), String>> {
            if self.polls_left > 0 {
                self.polls_left -= 1;
                return None;
            }
            Some(if self.fails {
                Err("terminó con código 1: no such service".into())
            } else {
                Ok(())
            })
        }
    }

    /// Reloj de mentira: dormir solo lo adelanta. La web responde a partir de
    /// `ready_at`; el puerto 5432 siempre acepta conexiones.
    struct FakeWaiter {
        clock: Cell<Duration>,
        ready_at: Option<Duration>,
    }

    impl FakeWaiter {
        fn ready_after(secs: Option<u64>) -> FakeWaiter {
            FakeWaiter {
                clock: Cell::new(Duration::ZERO),
                ready_at: secs.map(Duration::from_secs),
            }
        }
    }

    impl Waiter for FakeWaiter {
        fn web_responds(&self, _: &str) -> bool {
            self.ready_at.is_some_and(|t| self.clock.get() >= t)
        }
        fn accepts_connections(&self, port: u16) -> bool {
            port == 5432
        }
        fn sleep(&self, d: Duration) {
            self.clock.set(self.clock.get() + d);
        }
        fn now(&self) -> Duration {
            self.clock.get()
        }
    }

    /// Progreso que apunta los pasos y se cancela al llevar `cancel_at` segundos
    /// esperando.
    #[derive(Default)]
    struct Steps {
        seen: RefCell<Vec<(u32, Option<u32>)>>,
        cancel_at: Option<u32>,
        cancel: Cell<bool>,
    }

    impl Progress for Steps {
        fn step(&self, index: u32, _: &str, waited: Option<u32>) {
            self.seen.borrow_mut().push((index, waited));
            if waited.is_some() && waited >= self.cancel_at {
                self.cancel.set(true);
            }
        }
        fn cancelled(&self) -> bool {
            self.cancel.get()
        }
    }

    fn wait_for(until: WaitUntil, timeout_secs: u32, on_timeout: OnTimeout) -> Action {
        Action::Wait(WaitSpec {
            until,
            timeout_secs,
            on_timeout,
        })
    }

    fn web(url: &str) -> WaitUntil {
        WaitUntil::WebResponds { url: url.into() }
    }

    fn run(actions: &[Action], rec: &Recorder, waiter: &FakeWaiter) -> Vec<ActionOutcome> {
        run_actions(actions, rec, waiter, None, &NoProgress)
    }

    fn cmd(program: &str, args: &[&str]) -> CommandSpec {
        CommandSpec {
            program: program.into(),
            args: args.iter().map(|s| s.to_string()).collect(),
            cwd: None,
        }
    }

    #[test]
    fn runs_in_order_and_keeps_going_after_errors() {
        let dir = tempfile::tempdir().unwrap();
        let rec = Recorder {
            fail_urls: true,
            ..Default::default()
        };
        let actions = vec![
            Action::OpenUrl {
                url: "https://campus.es".into(),
                place: None,
            },
            Action::OpenFolder {
                path: dir.path().to_path_buf(),
                place: None,
            },
            Action::OpenApp {
                app_id: "Microsoft.WindowsCalculator_8wekyb3d8bbwe!App".into(),
                name: "Calculadora".into(),
                place: None,
            },
        ];
        let out = run(&actions, &rec, &FakeWaiter::ready_after(None));
        assert_eq!(rec.calls.borrow().len(), 3);
        assert_eq!(out[0].error.as_deref(), Some("sin navegador"));
        assert_eq!(out[1].error, None);
        assert_eq!(out[2].label, "Abrir Calculadora");
        assert!(out.iter().all(|o| o.ran));
        assert_eq!(
            out.iter().map(|o| o.index).collect::<Vec<_>>(),
            vec![1, 2, 3]
        );
    }

    #[test]
    fn missing_paths_are_reported_without_calling_the_os() {
        let rec = Recorder::default();
        let r = run_action(
            &Action::OpenFile {
                path: PathBuf::from(r"C:\no\existe.pdf"),
                place: None,
            },
            &rec,
        );
        assert!(r.unwrap_err().contains("no existe"));
        assert!(rec.calls.borrow().is_empty());
    }

    #[test]
    fn invalid_actions_never_reach_the_os() {
        // Aunque un JSON importado traiga algo peligroso, se bloquea antes de lanzarlo.
        let rec = Recorder::default();
        assert!(
            run_action(
                &Action::OpenUrl {
                    url: "file:///C:/x.exe".into(),
                    place: None,
                },
                &rec
            )
            .is_err()
        );
        assert!(
            run_action(
                &Action::OpenFile {
                    path: PathBuf::from(r"C:\x\a.exe"),
                    place: None,
                },
                &rec
            )
            .is_err()
        );
        assert!(rec.calls.borrow().is_empty());
    }

    #[test]
    fn command_arguments_stay_separate() {
        // Un argumento con "&" o comillas llega tal cual, como UN argumento: no hay
        // shell que lo interprete como "y ahora ejecuta esto otro".
        let rec = Recorder::default();
        let tricky = cmd("docker", &["compose", "up", "a & del C:\\*", "\"x\""]);
        run_action(&Action::RunCommand(tricky.clone()), &rec).unwrap();
        assert_eq!(rec.spawned.borrow()[0], tricky);
        assert_eq!(rec.spawned.borrow()[0].args.len(), 4);
    }

    #[test]
    fn missing_working_directory_is_an_error() {
        let rec = Recorder::default();
        let mut c = cmd("docker", &[]);
        c.cwd = Some(PathBuf::from(r"C:\no\existe"));
        assert!(
            run_action(&Action::RunCommand(c), &rec)
                .unwrap_err()
                .contains("carpeta de trabajo")
        );
        assert!(rec.spawned.borrow().is_empty());
    }

    #[test]
    fn pending_approvals_skips_approved_and_duplicates() {
        let up = cmd("docker", &["compose", "up"]);
        let down = cmd("docker", &["compose", "down"]);
        let actions = vec![
            Action::RunCommand(up.clone()),
            Action::OpenUrl {
                url: "https://x.es".into(),
                place: None,
            },
            Action::RunCommand(down.clone()),
            Action::RunCommand(up.clone()),
        ];
        assert_eq!(
            pending_approvals(&actions, |_| false),
            vec![up.clone(), down.clone()]
        );
        assert_eq!(pending_approvals(&actions, |c| *c == up), vec![down]);
        assert!(pending_approvals(&actions, |_| true).is_empty());
    }

    #[test]
    fn displays_commands_unambiguously() {
        let c = cmd(r"C:\Program Files\x.exe", &["--name", "mi carpeta", ""]);
        assert_eq!(
            display_command(&c),
            r#""C:\Program Files\x.exe" --name "mi carpeta" """#
        );
    }

    #[test]
    fn waits_until_the_web_responds_then_goes_on() {
        let rec = Recorder::default();
        let waiter = FakeWaiter::ready_after(Some(12));
        let actions = vec![
            wait_for(web("http://localhost:8080"), 60, OnTimeout::Stop),
            Action::OpenUrl {
                url: "http://localhost:8080".into(),
                place: None,
            },
        ];
        let out = run(&actions, &rec, &waiter);
        assert!(out.iter().all(|o| o.ran && o.error.is_none()), "{out:?}");
        assert_eq!(waiter.now(), Duration::from_secs(12));
        assert_eq!(*rec.calls.borrow(), vec!["url:http://localhost:8080"]);
    }

    #[test]
    fn a_wait_that_times_out_stops_the_rest_and_says_so() {
        let rec = Recorder::default();
        let actions = vec![
            Action::OpenApp {
                app_id: "code".into(),
                name: "VS Code".into(),
                place: None,
            },
            wait_for(web("http://localhost:8080"), 30, OnTimeout::Stop),
            Action::OpenUrl {
                url: "http://localhost:8080".into(),
                place: None,
            },
        ];
        let out = run(&actions, &rec, &FakeWaiter::ready_after(None));
        assert_eq!(out.len(), 3);
        assert!(out[0].ran && out[0].error.is_none());
        assert_eq!(
            out[1].error.as_deref(),
            Some("http://localhost:8080 no respondió en 30 s")
        );
        assert!(!out[2].ran, "la web no se abre");
        // VS Code se abrió y ahí sigue: solo se lanzó una cosa, nada se cerró.
        assert_eq!(*rec.calls.borrow(), vec!["app:code"]);
    }

    #[test]
    fn a_wait_can_keep_going_when_it_times_out() {
        let rec = Recorder::default();
        let actions = vec![
            wait_for(web("http://localhost:8080"), 5, OnTimeout::Continue),
            Action::OpenUrl {
                url: "http://localhost:8080".into(),
                place: None,
            },
        ];
        let out = run(&actions, &rec, &FakeWaiter::ready_after(None));
        assert!(out[0].error.is_some());
        assert!(out[1].ran && out[1].error.is_none());
    }

    #[test]
    fn waits_for_a_service_port_and_for_seconds() {
        let rec = Recorder::default();
        let waiter = FakeWaiter::ready_after(None);
        let actions = vec![
            wait_for(
                WaitUntil::ServiceAccepts { port: 5432 },
                60,
                OnTimeout::Stop,
            ),
            wait_for(WaitUntil::Seconds { secs: 3 }, 60, OnTimeout::Stop),
        ];
        let out = run(&actions, &rec, &waiter);
        assert!(out.iter().all(|o| o.ran && o.error.is_none()), "{out:?}");
        assert_eq!(waiter.now(), Duration::from_secs(3));
    }

    #[test]
    fn waits_for_the_previous_command_launched_without_window() {
        let rec = Recorder {
            finishes_after: 4,
            ..Default::default()
        };
        let actions = vec![
            Action::RunCommand(cmd("docker", &["compose", "up", "-d"])),
            wait_for(WaitUntil::CommandFinished, 120, OnTimeout::Stop),
            Action::RunCommand(cmd("code", &["."])),
        ];
        let out = run(&actions, &rec, &FakeWaiter::ready_after(None));
        assert!(out.iter().all(|o| o.ran && o.error.is_none()), "{out:?}");
        // El que se espera, sin ventana y con su salida guardada; el otro, como siempre.
        assert_eq!(*rec.calls.borrow(), vec!["captured:docker", "spawn:code"]);
    }

    #[test]
    fn a_failing_command_stops_with_its_reason() {
        let rec = Recorder {
            command_fails: true,
            ..Default::default()
        };
        let actions = vec![
            Action::RunCommand(cmd("docker", &["compose", "up", "-d"])),
            wait_for(WaitUntil::CommandFinished, 120, OnTimeout::Stop),
            Action::OpenUrl {
                url: "http://localhost:8080".into(),
                place: None,
            },
        ];
        let out = run(&actions, &rec, &FakeWaiter::ready_after(None));
        assert!(out[1].error.as_deref().unwrap().contains("no such service"));
        assert!(!out[2].ran);
    }

    #[test]
    fn cancelling_stops_waiting_and_lists_what_did_not_run() {
        let rec = Recorder::default();
        let steps = Steps {
            cancel_at: Some(4),
            ..Default::default()
        };
        let actions = vec![
            Action::OpenApp {
                app_id: "code".into(),
                name: "VS Code".into(),
                place: None,
            },
            wait_for(web("http://localhost:8080"), 60, OnTimeout::Continue),
            Action::OpenUrl {
                url: "http://localhost:8080".into(),
                place: None,
            },
        ];
        let out = run_actions(&actions, &rec, &FakeWaiter::ready_after(None), None, &steps);
        assert!(out[0].ran);
        assert!(!out[1].ran);
        assert_eq!(out[1].error.as_deref(), Some("cancelado mientras esperaba"));
        // Aunque el paso diga «seguir igualmente», cancelar para.
        assert!(!out[2].ran);
        assert_eq!(*rec.calls.borrow(), vec!["app:code"]);
        // La UI se enteró de cada segundo de espera.
        assert!(steps.seen.borrow().contains(&(2, Some(3))));
    }

    #[test]
    fn invalid_waits_never_wait() {
        let rec = Recorder::default();
        let waiter = FakeWaiter::ready_after(Some(0));
        let actions = vec![wait_for(
            web("http://campus.example.edu"),
            60,
            OnTimeout::Stop,
        )];
        let out = run(&actions, &rec, &waiter);
        let reason = out[0].error.as_deref().unwrap();
        assert!(reason.contains("en tu equipo"), "{reason}");
        assert!(!reason.contains("  "), "sin espacios repetidos: {reason}");
        assert_eq!(waiter.now(), Duration::ZERO);
    }

    #[test]
    fn checking_a_wait_once() {
        let up = FakeWaiter::ready_after(Some(0));
        let down = FakeWaiter::ready_after(None);
        let spec = |until| WaitSpec {
            until,
            timeout_secs: 60,
            on_timeout: OnTimeout::Stop,
        };
        assert!(check_wait_now(&spec(web("http://localhost:3000")), &up).is_ok());
        assert!(
            check_wait_now(&spec(web("http://localhost:3000")), &down)
                .unwrap_err()
                .contains("no responde ahora mismo")
        );
        assert!(check_wait_now(&spec(WaitUntil::ServiceAccepts { port: 5432 }), &down).is_ok());
        assert!(check_wait_now(&spec(WaitUntil::ServiceAccepts { port: 1 }), &down).is_err());
        assert!(check_wait_now(&spec(WaitUntil::CommandFinished), &up).is_err());
    }

    // --- Colocar ventanas (Fase 6.5) ---

    use crate::placement::{
        MonitorChoice, MonitorInfo, Placement, Rect, WindowInfo, WindowTarget, Zone,
    };

    /// Ventanas de mentira: la nueva «aparece» tras `appear_after` consultas.
    struct FakeWindows {
        existing: Vec<WindowInfo>,
        new_window: Option<WindowInfo>,
        appear_after: Cell<u32>,
        placed: RefCell<Vec<(isize, Zone, String)>>,
        focused: RefCell<Vec<isize>>,
    }

    impl FakeWindows {
        fn new(
            existing: Vec<WindowInfo>,
            new_window: Option<WindowInfo>,
            appear_after: u32,
        ) -> Self {
            FakeWindows {
                existing,
                new_window,
                appear_after: Cell::new(appear_after),
                placed: RefCell::new(Vec::new()),
                focused: RefCell::new(Vec::new()),
            }
        }
    }

    impl WindowManager for FakeWindows {
        fn windows(&self) -> Vec<WindowInfo> {
            let mut all = self.existing.clone();
            match self.appear_after.get() {
                0 => all.extend(self.new_window.clone()),
                n => self.appear_after.set(n - 1),
            }
            all
        }
        fn monitors(&self) -> Vec<MonitorInfo> {
            let bounds = Rect {
                left: 0,
                top: 0,
                right: 1920,
                bottom: 1080,
            };
            vec![MonitorInfo {
                name: "DELL".into(),
                bounds,
                work: bounds,
                primary: true,
            }]
        }
        fn mouse(&self) -> Option<(i32, i32)> {
            None
        }
        fn target(&self, _: &Action) -> WindowTarget {
            WindowTarget {
                exe: Some("code.exe".into()),
                title_hint: None,
            }
        }
        fn place(&self, window: isize, zone: Zone, monitor: &MonitorInfo) -> Result<(), String> {
            self.placed
                .borrow_mut()
                .push((window, zone, monitor.name.clone()));
            Ok(())
        }
        fn focus(&self, window: isize) -> Result<(), String> {
            self.focused.borrow_mut().push(window);
            Ok(())
        }
    }

    fn code(zone: Zone, reuse_window: bool) -> Action {
        code_in(Some(zone), reuse_window)
    }

    fn code_in(zone: Option<Zone>, reuse_window: bool) -> Action {
        Action::OpenApp {
            app_id: "code".into(),
            name: "VS Code".into(),
            place: Some(Placement {
                zone,
                monitor: MonitorChoice::Primary,
                reuse_window,
            }),
        }
    }

    fn code_window(id: isize) -> WindowInfo {
        WindowInfo {
            id,
            exe: "code.exe".into(),
            title: "Visual Studio Code".into(),
        }
    }

    #[test]
    fn opens_then_places_the_new_window_when_it_appears() {
        let rec = Recorder::default();
        let wm = FakeWindows::new(vec![code_window(1)], Some(code_window(2)), 6);
        let out = run_actions(
            &[code(Zone::RightHalf, false)],
            &rec,
            &FakeWaiter::ready_after(None),
            Some(&wm),
            &NoProgress,
        );
        assert!(
            out[0].ran && out[0].error.is_none() && out[0].note.is_none(),
            "{out:?}"
        );
        assert_eq!(*rec.calls.borrow(), vec!["app:code"]);
        // La nueva (2), no la que ya estaba abierta (1).
        assert_eq!(
            *wm.placed.borrow(),
            vec![(2, Zone::RightHalf, "DELL".to_string())]
        );
    }

    #[test]
    fn if_the_window_never_appears_it_goes_on_and_says_so() {
        let rec = Recorder::default();
        let wm = FakeWindows::new(vec![], None, 0);
        let waiter = FakeWaiter::ready_after(None);
        let actions = [
            code(Zone::LeftHalf, false),
            Action::OpenUrl {
                url: "https://x.es".into(),
                place: None,
            },
        ];
        let out = run_actions(&actions, &rec, &waiter, Some(&wm), &NoProgress);
        assert!(out[0].ran && out[0].error.is_none());
        assert!(
            out[0]
                .note
                .as_deref()
                .unwrap()
                .contains("no encontré su ventana")
        );
        assert_eq!(waiter.now(), WINDOW_TIMEOUT);
        // Seguir y avisar: lo siguiente se abre igual.
        assert!(out[1].ran && out[1].error.is_none());
    }

    #[test]
    fn an_already_open_window_is_moved_instead_of_opening_another() {
        let rec = Recorder::default();
        let wm = FakeWindows::new(vec![code_window(7)], None, 0);
        let out = run_actions(
            &[code(Zone::Maximized, true)],
            &rec,
            &FakeWaiter::ready_after(None),
            Some(&wm),
            &NoProgress,
        );
        assert!(out[0].error.is_none());
        assert!(rec.calls.borrow().is_empty(), "no se abre otra");
        assert_eq!(
            *wm.placed.borrow(),
            vec![(7, Zone::Maximized, "DELL".to_string())]
        );
    }

    #[test]
    fn without_a_zone_an_open_window_is_brought_to_front_without_moving_it() {
        let rec = Recorder::default();
        let wm = FakeWindows::new(vec![code_window(7)], None, 0);
        let out = run_actions(
            &[code_in(None, true)],
            &rec,
            &FakeWaiter::ready_after(None),
            Some(&wm),
            &NoProgress,
        );
        assert!(out[0].error.is_none() && out[0].note.is_none(), "{out:?}");
        assert!(rec.calls.borrow().is_empty(), "no se abre otra");
        assert_eq!(*wm.focused.borrow(), vec![7]);
        assert!(wm.placed.borrow().is_empty(), "no se mueve");
    }

    #[test]
    fn without_a_zone_and_nothing_open_it_opens_without_waiting_for_the_window() {
        let rec = Recorder::default();
        let wm = FakeWindows::new(vec![], None, 0);
        let waiter = FakeWaiter::ready_after(None);
        let out = run_actions(
            &[code_in(None, true)],
            &rec,
            &waiter,
            Some(&wm),
            &NoProgress,
        );
        assert!(out[0].error.is_none() && out[0].note.is_none(), "{out:?}");
        assert_eq!(*rec.calls.borrow(), vec!["app:code"]);
        assert_eq!(waiter.now(), Duration::ZERO, "no espera a su ventana");
        assert!(wm.focused.borrow().is_empty() && wm.placed.borrow().is_empty());
    }

    #[test]
    fn saved_placements_without_zone_field_still_load() {
        // Los Espacios guardados antes siempre tienen zona; uno sin zona es «no mover».
        let old: Placement =
            serde_json::from_str(r#"{"zone":"leftHalf","monitor":{"kind":"primary"}}"#).unwrap();
        assert_eq!(old.zone, Some(Zone::LeftHalf));
        assert!(!old.reuse_window);
        let new: Placement = serde_json::from_str(r#"{"reuseWindow":true}"#).unwrap();
        assert_eq!(new.zone, None);
        assert!(new.reuse_window);
    }

    #[test]
    fn without_a_window_manager_it_just_opens() {
        let rec = Recorder::default();
        let out = run_actions(
            &[code(Zone::LeftHalf, false)],
            &rec,
            &FakeWaiter::ready_after(None),
            None,
            &NoProgress,
        );
        assert!(out[0].error.is_none() && out[0].note.is_none());
        assert_eq!(*rec.calls.borrow(), vec!["app:code"]);
    }

    #[test]
    fn cancelling_while_looking_for_the_window_stops_the_rest() {
        let rec = Recorder::default();
        let wm = FakeWindows::new(vec![], None, 0);
        let steps = Steps {
            cancel_at: Some(2),
            ..Default::default()
        };
        let actions = [
            code(Zone::LeftHalf, false),
            Action::OpenUrl {
                url: "https://x.es".into(),
                place: None,
            },
        ];
        let out = run_actions(
            &actions,
            &rec,
            &FakeWaiter::ready_after(None),
            Some(&wm),
            &steps,
        );
        assert!(out[0].ran);
        assert!(out[0].note.as_deref().unwrap().contains("cancelado"));
        assert!(!out[1].ran);
        assert_eq!(*rec.calls.borrow(), vec!["app:code"]);
    }

    #[test]
    fn the_approval_message_shows_each_command_and_where() {
        let mut up = cmd("docker", &["compose", "up", "-d"]);
        up.cwd = Some(PathBuf::from(r"C:\Proyecto"));
        let one = approval_message("Bases de datos", std::slice::from_ref(&up));
        assert!(one.starts_with("«Bases de datos» va a ejecutar un comando por primera vez:"));
        assert!(one.contains("docker compose up -d"));
        assert!(one.contains(r"en C:\Proyecto"));
        assert!(!one.contains("  Solo"), "sin espacios de más: {one}");
        let two = approval_message("X", &[up, cmd("code", &["."])]);
        assert!(two.contains("estos comandos"));
    }
}
