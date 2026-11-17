// La versión de DESARROLLO de Sift, manejada por el protocolo de DevTools del WebView2:
// arrancarla, conectarse a sus páginas (evaluar JavaScript, capturas, errores de la
// consola) y cerrarla. La usan scripts/e2e.mjs y scripts/capturas.mjs.
//
// Usa sus propios datos de prueba (`%APPDATA%\dev.sift.app-dev`), nunca los de la versión
// diaria. Abre ventanas en la pantalla.

import { spawn, execFileSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

export const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const desktop = path.join(root, "apps", "desktop");
const PORT = 9223;

export const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

/** Arranca `npm run tauri dev` con el puerto de depuración del WebView2. */
export function startDevApp() {
  // `npm` es un .cmd: hace falta cmd.exe. La orden es fija, sin nada que venga de fuera.
  const app = spawn(process.env.ComSpec ?? "cmd.exe", ["/d", "/s", "/c", "npm run tauri dev"], {
    cwd: desktop,
    env: {
      ...process.env,
      WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: `--remote-debugging-port=${PORT}`,
    },
    stdio: ["ignore", "pipe", "pipe"],
  });
  const state = { log: "" };
  app.stdout.on("data", (d) => (state.log += d));
  app.stderr.on("data", (d) => (state.log += d));
  return {
    get log() {
      return state.log;
    },
    stop() {
      try {
        execFileSync("taskkill", ["/T", "/F", "/PID", String(app.pid)], { stdio: "ignore" });
      } catch {
        // ya había terminado
      }
    },
  };
}

async function targets() {
  try {
    return await (await fetch(`http://127.0.0.1:${PORT}/json`)).json();
  } catch {
    return [];
  }
}

export async function waitFor(fn, ms, what) {
  const until = Date.now() + ms;
  while (Date.now() < until) {
    const v = await fn();
    if (v) return v;
    await sleep(500);
  }
  throw new Error(`no llegó a tiempo: ${what}`);
}

/** Conexión a una página: evaluar JavaScript, capturas y errores de la consola. */
export async function connect(urlPart) {
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
  /** Captura de la página en `file` (png o jpg, según la extensión). */
  const shot = async (file) => {
    const jpeg = /\.jpe?g$/i.test(file);
    const r = await send(
      "Page.captureScreenshot",
      jpeg ? { format: "jpeg", quality: 82 } : { format: "png" },
    );
    fs.mkdirSync(path.dirname(file), { recursive: true });
    fs.writeFileSync(file, Buffer.from(r.result.data, "base64"));
  };
  return { evaluate, shot, errors, close: () => ws.close() };
}

/** Escribe en la caja de la barra como lo haría una persona (dispara `input`). */
export function typeInBar(bar, text) {
  return bar.evaluate(`(() => {
    const box = document.querySelector('input[aria-label="Buscar"]');
    const set = Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, 'value').set;
    set.call(box, ${JSON.stringify(text)});
    box.dispatchEvent(new Event('input', { bubbles: true }));
  })()`);
}
