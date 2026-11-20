// Lo común de las pruebas de aguante (carga.mjs, largo.mjs, robustez.mjs): arrancar la
// compilación optimizada de pruebas (`--profile aguante`, ver Cargo.toml), que es «desarrollo»
// para Sift y usa SUS datos (`%APPDATA%\dev.sift.app-dev`), nunca los de la versión diaria;
// apartar esos datos de desarrollo y devolverlos al acabar; conectarse por DevTools; y medir
// memoria.
//
// Compilar antes (desde la raíz):
//   cd apps/desktop && npm run build && cd ../..
//   cargo build -p sift-desktop --profile aguante --features tauri/custom-protocol

import { execFileSync, spawn } from "node:child_process";
import fs from "node:fs";
import path from "node:path";
import { connect, root, sleep, waitFor } from "../devapp.mjs";

export { connect, root, sleep, waitFor };

export const EXE = path.join(root, "target", "aguante", "sift-desktop.exe");
export const DEV_DATA = path.join(process.env.APPDATA, "dev.sift.app-dev");
const BACKUP = `${DEV_DATA}.antes-aguante`;
const PORT = 9223;

/** Aparta los datos de desarrollo (para empezar de cero) y devuelve cómo dejarlos como estaban. */
export function setAsideDevData() {
  if (fs.existsSync(BACKUP)) throw new Error(`ya existe ${BACKUP}: revísalo a mano antes de seguir`);
  const had = fs.existsSync(DEV_DATA);
  if (had) fs.renameSync(DEV_DATA, BACKUP);
  return () => {
    if (fs.existsSync(DEV_DATA)) fs.rmSync(DEV_DATA, { recursive: true, force: true });
    if (had) fs.renameSync(BACKUP, DEV_DATA);
    return fs.existsSync(DEV_DATA) === had;
  };
}

/** Arranca la compilación de pruebas leyendo solo `folder` (SIFT_DEV_ROOT). */
export function startSift(folder, env = {}) {
  if (!fs.existsSync(EXE)) throw new Error(`falta ${EXE}: compílalo (ver cabecera)`);
  const p = spawn(EXE, [], {
    env: {
      ...process.env,
      ...env,
      ...(folder ? { SIFT_DEV_ROOT: folder } : {}),
      WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: `--remote-debugging-port=${PORT}`,
    },
    stdio: "ignore",
    detached: false,
  });
  return {
    pid: p.pid,
    /** Cierre normal (como Salir): mata el árbol de procesos de forma ordenada. */
    stop() {
      try {
        execFileSync("taskkill", ["/T", "/PID", String(p.pid)], { stdio: "ignore" });
      } catch {}
      try {
        execFileSync("taskkill", ["/T", "/F", "/PID", String(p.pid)], { stdio: "ignore" });
      } catch {}
    },
    /** Cierre brusco (como un corte de luz o «Finalizar tarea»). */
    kill() {
      try {
        execFileSync("taskkill", ["/T", "/F", "/PID", String(p.pid)], { stdio: "ignore" });
      } catch {}
    },
  };
}

/** La barra (página principal) de la compilación de pruebas. */
export const connectBar = () => connect("tauri.localhost/$");

export const invoke = (page, cmd, args = {}) =>
  page.evaluate(`window.__TAURI_INTERNALS__.invoke(${JSON.stringify(cmd)}, ${JSON.stringify(args)})`);

/**
 * Memoria en MB: el proceso de Sift (el núcleo en Rust) y la suma de los procesos de su
 * WebView2 (los que usan su carpeta de datos).
 */
export function memory(pid) {
  const out = execFileSync(
    "powershell",
    [
      "-NoProfile",
      "-Command",
      `$s = Get-Process -Id ${pid} -ErrorAction SilentlyContinue; ` +
        `$w = Get-CimInstance Win32_Process -Filter "Name='msedgewebview2.exe'" | Where-Object { $_.CommandLine -like '*dev.sift.app-dev*' }; ` +
        `$ws = ($w | Measure-Object -Property WorkingSetSize -Sum).Sum; ` +
        `"{0}|{1}|{2}" -f $s.WorkingSet64, $s.PrivateMemorySize64, $ws`,
    ],
    { encoding: "utf8" },
  ).trim();
  const [ws, priv, web] = out.split("|").map((n) => Number(n || 0) / 2 ** 20);
  return { siftWorkingMb: Math.round(ws), siftPrivateMb: Math.round(priv), webviewMb: Math.round(web) };
}

/** Tamaño en MB de una carpeta (recursivo). */
export function folderMb(dir) {
  let total = 0;
  const walk = (d) => {
    for (const e of fs.readdirSync(d, { withFileTypes: true })) {
      const p = path.join(d, e.name);
      if (e.isDirectory()) walk(p);
      else total += fs.statSync(p).size;
    }
  };
  if (fs.existsSync(dir)) walk(dir);
  return Math.round((total / 2 ** 20) * 10) / 10;
}

export const percentile = (xs, p) => {
  const s = [...xs].sort((a, b) => a - b);
  return s.length ? Math.round(s[Math.min(s.length - 1, Math.floor((p / 100) * s.length))] * 10) / 10 : null;
};

/** Espera a que el indexado acabe, apuntando el progreso y la memoria. */
export async function waitIndexed(bar, pid, { timeoutMs = 4 * 3600_000, every = 5000, log = () => {} } = {}) {
  const t0 = Date.now();
  const samples = [];
  let peak = { siftWorkingMb: 0, webviewMb: 0 };
  // Primero, que empiece (la primera lectura tarda un poco en arrancar).
  await sleep(3000);
  for (;;) {
    const s = await invoke(bar, "index_status");
    const m = memory(pid);
    peak = { siftWorkingMb: Math.max(peak.siftWorkingMb, m.siftWorkingMb), webviewMb: Math.max(peak.webviewMb, m.webviewMb) };
    samples.push({ t: Math.round((Date.now() - t0) / 1000), done: s.done, total: s.total, phase: s.phase, ...m });
    log(`${Math.round((Date.now() - t0) / 1000)} s · ${s.phase} ${s.done}/${s.total} · ${m.siftWorkingMb} MB`);
    // Acabado: en reposo y con una revisión completa terminada en esta sesión (`last`).
    if (!s.running && s.phase === "idle" && s.last) break;
    if (Date.now() - t0 > timeoutMs) throw new Error("el indexado no acabó a tiempo");
    await sleep(every);
  }
  return { seconds: Math.round((Date.now() - t0) / 1000), peak, samples };
}
