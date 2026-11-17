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

import fs from "node:fs";
import path from "node:path";
import { connect, root, sleep, startDevApp, typeInBar, waitFor } from "./devapp.mjs";

const out = path.join(root, "target", "e2e");
fs.mkdirSync(out, { recursive: true });

const results = [];
const check = (name, ok, detail = "") => {
  results.push({ name, ok, detail });
  console.log(`${ok ? "OK   " : "FALLA"} ${name}${detail ? ` · ${detail}` : ""}`);
};

const app = startDevApp();
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
  await typeInBar(bar, "vidrio");
  const found = await waitFor(
    () =>
      bar.evaluate(
        "[...document.querySelectorAll('[role=option]')].map(o => o.textContent).join(' | ')",
      ),
    30_000,
    "resultados de «vidrio»",
  ).catch(() => "");
  check("Buscar «vidrio» trae la guía de diseño", found.includes("design-system"), found.slice(0, 120));
  await bar.shot(path.join(out, "barra.png"));

  // 3. Ajustes.
  await bar.evaluate("window.__TAURI_INTERNALS__.invoke('open_settings', { newSpace: false })");
  const settings = await connect("settings.html");
  const heading = await waitFor(
    () => settings.evaluate("document.querySelector('h1')?.textContent"),
    20_000,
    "la cabecera de Ajustes",
  ).catch(() => null);
  check("Ajustes se abre", heading === "Ajustes", String(heading));
  await settings.shot(path.join(out, "ajustes.png"));

  // 4. Sin errores de JavaScript en ninguna de las dos.
  const errors = [...bar.errors, ...settings.errors];
  check("Sin errores de JavaScript", errors.length === 0, errors.join(" ; ").slice(0, 200));
  bar.close();
  settings.close();
} catch (e) {
  check("La prueba llegó al final", false, String(e));
  fs.writeFileSync(path.join(out, "app.log"), app.log);
} finally {
  app.stop();
  failed = results.some((r) => !r.ok);
  console.log(`\nCapturas en ${out}`);
  console.log(failed ? "E2E en ROJO" : "E2E en VERDE");
  process.exit(failed ? 1 : 0);
}
