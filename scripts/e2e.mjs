// Prueba de humo de extremo a extremo: la app de verdad, manejada por el protocolo de
// DevTools del WebView2 (sin ratón ni teclado). Auditoría, F20.
//
// Uso (desde la raíz del repo; abre ventanas en la pantalla):
//   node scripts/e2e.mjs
//
// Arranca la versión de DESARROLLO (`npm run tauri dev`): usa sus propios datos de prueba
// (`%APPDATA%\dev.sift.app-dev`), nunca los de la versión diaria. Comprueba:
//   1. la barra carga y habla con Rust (IPC): está el Espacio de prueba;
//   2. una búsqueda en la barra trae resultados del índice (la carpeta docs/);
//   3. Ajustes se abre;
//   4. ninguna página tiene errores de JavaScript.
// Guarda capturas en target/e2e/ y cierra todo al acabar. Sale con 1 si algo falla.

import { spawn, execFileSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const desktop = path.join(root, "apps", "desktop");
const out = path.join(root, "target", "e2e");
const PORT = 9223;
fs.mkdirSync(out, { recursive: true });

const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
const results = [];
const check = (name, ok, detail = "") => {
  results.push({ name, ok, detail });
  console.log(`${ok ? "OK   " : "FALLA"} ${name}${detail ? ` · ${detail}` : ""}`);
};

// --- Arrancar la app de desarrollo con el puerto de depuración del WebView2 ---
// `npm` es un .cmd: hace falta cmd.exe. La orden es fija, sin nada que venga de fuera.
const app = spawn(process.env.ComSpec ?? "cmd.exe", ["/d", "/s", "/c", "npm run tauri dev"], {
  cwd: desktop,
  env: {
    ...process.env,
    WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: `--remote-debugging-port=${PORT}`,
  },
  stdio: ["ignore", "pipe", "pipe"],
});
let log = "";
app.stdout.on("data", (d) => (log += d));
app.stderr.on("data", (d) => (log += d));

function stopApp() {
  try {
    execFileSync("taskkill", ["/T", "/F", "/PID", String(app.pid)], { stdio: "ignore" });
  } catch {
    // ya había terminado
  }
}

async function targets() {
  try {
    return await (await fetch(`http://127.0.0.1:${PORT}/json`)).json();
  } catch {
    return [];
  }
}

async function waitFor(fn, ms, what) {
  const until = Date.now() + ms;
  while (Date.now() < until) {
    const v = await fn();
    if (v) return v;
    await sleep(500);
  }
  throw new Error(`no llegó a tiempo: ${what}`);
}

/** Conexión a una página: evaluar JavaScript, capturas y errores de la consola. */
async function connect(urlPart) {
  const t = await waitFor(
    async () => (await targets()).find((x) => x.type === "page" && x.url.includes(urlPart)),
    180_000,
    `la página ${urlPart}`,
  );
  const ws = new WebSocket(t.webSocketDebuggerUrl);
  const pending = new Map();
  const errors = [];
  let id = 0;
  ws.onmessage = (m) => {
    const msg = JSON.parse(m.data);
    if (msg.id && pending.has(msg.id)) pending.get(msg.id)(msg);
    if (msg.method === "Runtime.exceptionThrown")
      errors.push(msg.params.exceptionDetails.exception?.description ?? "error");
    if (msg.method === "Runtime.consoleAPICalled" && msg.params.type === "error")
      errors.push(msg.params.args.map((a) => a.value ?? a.description).join(" "));
  };
  await new Promise((r) => (ws.onopen = r));
  const send = (method, params = {}) =>
    new Promise((resolve) => {
      const n = ++id;
      pending.set(n, resolve);
      ws.send(JSON.stringify({ id: n, method, params }));
    });
  await send("Runtime.enable");
  const evaluate = async (expression) => {
    const r = await send("Runtime.evaluate", {
      expression,
      awaitPromise: true,
      returnByValue: true,
    });
    if (r.result?.exceptionDetails) throw new Error(r.result.exceptionDetails.text);
    return r.result?.result?.value;
  };
  const shot = async (name) => {
    const r = await send("Page.captureScreenshot", { format: "png" });
    fs.writeFileSync(path.join(out, name), Buffer.from(r.result.data, "base64"));
  };
  return { evaluate, shot, errors, close: () => ws.close() };
}

let failed = false;
try {
  console.log("Arrancando la versión de desarrollo (la primera vez compila: unos minutos)…");
  const bar = await connect("localhost:1420/");
  await sleep(1500);

  // 1. IPC: el Espacio de prueba de la versión de desarrollo.
  const spaces = await bar.evaluate(
    "window.__TAURI_INTERNALS__.invoke('list_spaces').then(l => l.map(v => v.space.name))",
  );
  check("La barra habla con Rust", Array.isArray(spaces), JSON.stringify(spaces));
  check(
    "Son los datos de prueba, no los tuyos",
    Array.isArray(spaces) && spaces.includes("Sift (desarrollo)"),
  );

  // 2. Buscar como lo haría una persona: escribir en la caja y esperar resultados.
  await bar.evaluate(`(() => {
    const box = document.querySelector('input[aria-label="Buscar"]');
    const set = Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, 'value').set;
    set.call(box, 'vidrio');
    box.dispatchEvent(new Event('input', { bubbles: true }));
  })()`);
  const found = await waitFor(
    () =>
      bar.evaluate(
        "[...document.querySelectorAll('[role=option]')].map(o => o.textContent).join(' | ')",
      ),
    30_000,
    "resultados de «vidrio»",
  ).catch(() => "");
  check("Buscar «vidrio» trae la guía de diseño", found.includes("design-system"), found.slice(0, 120));
  await bar.shot("barra.png");

  // 3. Ajustes.
  await bar.evaluate("window.__TAURI_INTERNALS__.invoke('open_settings', { newSpace: false })");
  const settings = await connect("settings.html");
  const heading = await waitFor(
    () => settings.evaluate("document.querySelector('h1')?.textContent"),
    20_000,
    "la cabecera de Ajustes",
  ).catch(() => null);
  check("Ajustes se abre", heading === "Ajustes", String(heading));
  await settings.shot("ajustes.png");

  // 4. Sin errores de JavaScript en ninguna de las dos.
  const errors = [...bar.errors, ...settings.errors];
  check("Sin errores de JavaScript", errors.length === 0, errors.join(" ; ").slice(0, 200));
  bar.close();
  settings.close();
} catch (e) {
  check("La prueba llegó al final", false, String(e));
  fs.writeFileSync(path.join(out, "app.log"), log);
} finally {
  stopApp();
  failed = results.some((r) => !r.ok);
  console.log(`\nCapturas en ${out}`);
  console.log(failed ? "E2E en ROJO" : "E2E en VERDE");
  process.exit(failed ? 1 : 0);
}
