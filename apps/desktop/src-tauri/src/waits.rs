//! Las esperas de los Espacios, en Windows (ADR 0012): comprobar si una web o un servicio
//! de tu equipo están listos, y lanzar un comando sin ventana guardando su salida para
//! poder esperar a que termine.

use std::collections::VecDeque;
use std::io::{BufRead, BufReader, Read};
use std::net::{SocketAddr, TcpStream};
use std::process::Stdio;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use sift_spaces::{Child, CommandSpec, Waiter};

/// Cuánto se espera a cada comprobación (cada medio segundo se vuelve a intentar).
const CHECK_TIMEOUT: Duration = Duration::from_secs(2);
const CONNECT_TIMEOUT: Duration = Duration::from_millis(400);
/// Líneas de salida que se guardan de un comando (las últimas) y cuántas se enseñan.
const KEEP_LINES: usize = 40;
const SHOW_LINES: usize = 6;

/// El `Waiter` de verdad.
pub struct LocalWaiter {
    started: Instant,
    agent: ureq::Agent,
}

impl LocalWaiter {
    pub fn new() -> LocalWaiter {
        let agent = ureq::Agent::config_builder()
            .timeout_global(Some(CHECK_TIMEOUT))
            // Solo se mira si responde: sin seguir redirecciones (podrían llevar fuera
            // del equipo) ni proxies del sistema (el aviso iría a otro sitio).
            .max_redirects(0)
            .proxy(None)
            // Un 404 también es «responder»: el servidor ya está en marcha.
            .http_status_as_error(false)
            .build()
            .into();
        LocalWaiter {
            started: Instant::now(),
            agent,
        }
    }
}

impl Waiter for LocalWaiter {
    fn web_responds(&self, url: &str) -> bool {
        // Doble seguro: la validación ya solo deja direcciones de este equipo.
        if !sift_spaces::validate::is_local_web(url) {
            return false;
        }
        match self.agent.get(url).header("User-Agent", "Sift").call() {
            // Un 5xx suele ser «arrancando» (o un proxy sin el servicio detrás).
            Ok(response) => response.status().as_u16() < 500,
            Err(_) => false,
        }
    }

    fn accepts_connections(&self, port: u16) -> bool {
        ["127.0.0.1", "[::1]"].iter().any(|host| {
            format!("{host}:{port}")
                .parse::<SocketAddr>()
                .is_ok_and(|addr| TcpStream::connect_timeout(&addr, CONNECT_TIMEOUT).is_ok())
        })
    }

    fn sleep(&self, d: Duration) {
        std::thread::sleep(d);
    }

    fn now(&self) -> Duration {
        self.started.elapsed()
    }
}

/// Un comando lanzado sin ventana, con su salida guardada.
pub struct CapturedChild {
    program: String,
    child: std::process::Child,
    lines: Arc<Mutex<VecDeque<String>>>,
}

/// Lanza un comando sin ventana y guarda sus últimas líneas de salida (y de error).
/// Programa y argumentos por separado, sin shell, como siempre.
pub fn spawn_captured(command: &CommandSpec) -> Result<Box<dyn Child>, String> {
    let mut cmd = std::process::Command::new(&command.program);
    cmd.args(&command.args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if let Some(cwd) = &command.cwd {
        cmd.current_dir(cwd);
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    let mut child = cmd
        .spawn()
        .map_err(|e| format!("no se pudo ejecutar «{}»: {e}", command.program))?;
    let lines = Arc::new(Mutex::new(VecDeque::new()));
    // Hay que leer la salida mientras se produce: si nadie la lee, el comando se queda
    // bloqueado al llenar el búfer.
    if let Some(out) = child.stdout.take() {
        keep_last_lines(out, lines.clone());
    }
    if let Some(err) = child.stderr.take() {
        keep_last_lines(err, lines.clone());
    }
    Ok(Box::new(CapturedChild {
        program: command.program.clone(),
        child,
        lines,
    }))
}

fn keep_last_lines(stream: impl Read + Send + 'static, lines: Arc<Mutex<VecDeque<String>>>) {
    std::thread::spawn(move || {
        // `lossy`: la consola de Windows no siempre escribe UTF-8.
        let mut reader = BufReader::new(stream);
        let mut buf = Vec::new();
        while reader.read_until(b'\n', &mut buf).is_ok_and(|n| n > 0) {
            let line = String::from_utf8_lossy(&buf).trim_end().to_owned();
            buf.clear();
            if line.is_empty() {
                continue;
            }
            let mut lines = lines.lock().unwrap_or_else(|p| p.into_inner());
            if lines.len() == KEEP_LINES {
                lines.pop_front();
            }
            lines.push_back(line);
        }
    });
}

impl Child for CapturedChild {
    fn try_finished(&mut self) -> Option<Result<(), String>> {
        match self.child.try_wait() {
            Ok(None) => None,
            Ok(Some(status)) if status.success() => Some(Ok(())),
            Ok(Some(status)) => {
                // Un momento para que los hilos lean lo último que escribió.
                std::thread::sleep(Duration::from_millis(100));
                let lines = self.lines.lock().unwrap_or_else(|p| p.into_inner());
                let tail: Vec<&str> = lines
                    .iter()
                    .skip(lines.len().saturating_sub(SHOW_LINES))
                    .map(String::as_str)
                    .collect();
                let code = status
                    .code()
                    .map_or_else(|| "sin código".into(), |c| format!("código {c}"));
                let mut reason = format!("«{}» terminó con error ({code})", self.program);
                if !tail.is_empty() {
                    reason.push_str(":\n");
                    reason.push_str(&tail.join("\n"));
                }
                Some(Err(reason))
            }
            Err(e) => Some(Err(format!(
                "no se pudo saber si «{}» terminó: {e}",
                self.program
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::TcpListener;

    #[test]
    fn a_listening_port_accepts_connections() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let waiter = LocalWaiter::new();
        assert!(waiter.accepts_connections(port));
        drop(listener);
        assert!(!waiter.accepts_connections(port));
    }

    #[test]
    fn a_local_web_server_responds_and_remote_ones_are_never_asked() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        // Un servidor mínimo: contesta 404 a la primera petición.
        let server = std::thread::spawn(move || {
            use std::io::Write;
            let (mut s, _) = listener.accept().unwrap();
            let mut buf = [0u8; 1024];
            let _ = s.read(&mut buf);
            s.write_all(
                b"HTTP/1.1 404 Not Found\r\ncontent-length: 0\r\nconnection: close\r\n\r\n",
            )
            .unwrap();
        });
        let waiter = LocalWaiter::new();
        assert!(waiter.web_responds(&format!("http://127.0.0.1:{port}/salud")));
        server.join().unwrap();
        assert!(!waiter.web_responds("http://example.com"));
    }

    #[cfg(windows)]
    #[test]
    fn a_failing_command_reports_its_last_lines() {
        // `cmd /c` solo en el test, para tener un comando que escribe y falla en
        // cualquier Windows. La app nunca pasa nada por `cmd`.
        let mut child = spawn_captured(&CommandSpec {
            program: "cmd".into(),
            args: vec!["/c".into(), "echo hola && echo fallo 1>&2 && exit 3".into()],
            cwd: None,
        })
        .unwrap();
        let started = Instant::now();
        let result = loop {
            if let Some(r) = child.try_finished() {
                break r;
            }
            assert!(started.elapsed() < Duration::from_secs(10));
            std::thread::sleep(Duration::from_millis(50));
        };
        let reason = result.unwrap_err();
        assert!(reason.contains("código 3"), "{reason}");
        assert!(reason.contains("fallo"), "{reason}");
    }
}
