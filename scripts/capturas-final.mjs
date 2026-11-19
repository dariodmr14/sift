// Comprobación en pantalla de la Fase 7 y capturas finales de cada pantalla
// (docs/capturas/final/). Abre ventanas: no hay que tocar nada mientras tanto.
//
// Uso (desde la raíz del repo):
//   node scripts/capturas-final.mjs [--sin-bienvenida]
//
// Usa SOLO la versión de DESARROLLO (sus datos en %APPDATA%\dev.sift.app-dev,
// nunca los de la versión diaria). Dos arranques:
//   1. La bienvenida: aparta la carpeta de datos de desarrollo (la renombra), arranca sin
//      datos de prueba (SIFT_DEV_NO_SEED=1), captura, cierra, borra la carpeta nueva que
//      ha creado ese arranque y devuelve la de antes a su sitio.
//   2. Lo demás, con los DATOS DE DEMOSTRACIÓN (SIFT_DEV_DEMO=1: demo/ del repo, inventado,
//      copiado a Documentos públicos; ninguna ruta personal en pantalla), también sobre
//      una carpeta de datos apartada y devuelta: barra vacía, búsqueda, vista previa (teclado de
//      verdad por el protocolo de DevTools), animaciones, Ajustes, editor de Espacios y el
//      diálogo del icono (que atrape el foco y lo devuelva al cerrar).
// Sale con 1 si alguna comprobación falla.

import { execFileSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";
import { connect, root, searchUntil, sleep, startDevApp, typeInBar, waitFor } from "./devapp.mjs";

const out = path.join(root, "docs", "capturas", "final");
fs.mkdirSync(out, { recursive: true });
const appData = process.env.APPDATA;
const devData = path.join(appData, "dev.sift.app-dev");

const results = [];
const check = (name, ok, detail = "") => {
  results.push({ name, ok: Boolean(ok), detail });
  console.log(`${ok ? "OK   " : "FALLA"} ${name}${detail ? ` · ${detail}` : ""}`);
};

/** Captura la ventana de la página tal como se ve en pantalla (el vidrio lo pinta Windows). */
async function grab(page, name) {
  const r = await page.evaluate(`({
    x: Math.round(window.screenX * devicePixelRatio),
    y: Math.round(window.screenY * devicePixelRatio),
    w: Math.round(window.outerWidth * devicePixelRatio),
    h: Math.round(window.outerHeight * devicePixelRatio),
  })`);
  execFileSync("powershell", [
    "-NoProfile", "-ExecutionPolicy", "Bypass",
    "-File", path.join(root, "scripts", "captura-region.ps1"),
    "-X", String(r.x), "-Y", String(r.y), "-W", String(r.w), "-H", String(r.h),
    "-Out", path.join(out, name),
  ]);
}

/** Lleva el puntero real a la esquina de abajo a la derecha: así no deja un globo de ayuda
 *  de Windows encima de lo que se captura. */
function parkCursor() {
  execFileSync("powershell", [
    "-NoProfile",
    "-Command",
    "Add-Type -AssemblyName System.Windows.Forms; $b = [System.Windows.Forms.Screen]::PrimaryScreen.Bounds; [System.Windows.Forms.Cursor]::Position = New-Object System.Drawing.Point(($b.Right - 2), ($b.Bottom - 2))",
  ]);
}

async function showBar(bar) {
  const visible = () => bar.evaluate("document.visibilityState === 'visible' && document.hasFocus()");
  if (await visible()) return;
  execFileSync("powershell", ["-NoProfile", "-ExecutionPolicy", "Bypass", "-File", path.join(root, "scripts", "atajo-dev.ps1")]);
  await waitFor(visible, 5_000, "la barra a la vista");
  await sleep(400);
}

const text = (page, selector) =>
  page.evaluate(`document.querySelector(${JSON.stringify(selector)})?.textContent ?? null`);

/**
 * Arranca la versión de desarrollo sobre una carpeta de datos nueva: aparta la de ahora,
 * ejecuta `run(app)`, cierra, borra la nueva y devuelve la de antes a su sitio.
 */
async function withFreshData(env, run) {
  const backup = `${devData}.antes-capturas`;
  if (fs.existsSync(backup)) throw new Error(`ya existe ${backup}: revísalo a mano antes de seguir`);
  const had = fs.existsSync(devData);
  if (had) fs.renameSync(devData, backup);
  const app = startDevApp(env);
  try {
    await run(app);
  } finally {
    app.stop();
    await sleep(2500); // que suelte los archivos
    if (fs.existsSync(devData)) fs.rmSync(devData, { recursive: true, force: true });
    if (had) fs.renameSync(backup, devData);
  }
  check("La carpeta de datos de desarrollo queda como estaba", fs.existsSync(devData) === had);
}

// ---------- 1. Bienvenida (la introducción, paso 1) ----------
async function welcome() {
  await withFreshData({ SIFT_DEV_NO_SEED: "1", SIFT_DEV_DEMO: "1" }, async () => {
    const page = await connect("settings.html");
    const heading = await waitFor(
      () => text(page, "h1, h2"),
      30_000,
      "la bienvenida",
    ).catch(() => null);
    check("La bienvenida sale en un equipo nuevo", heading?.includes("Te damos la bienvenida a Sift"), String(heading));
    // El permiso de descarga va en el paso 4: lo comprueba scripts/capturas-intro.mjs.
    await sleep(600);
    await grab(page, "bienvenida.jpg");
    check("Sin errores de JavaScript en la bienvenida", page.errors.length === 0, page.errors.join(" ; ").slice(0, 200));
    page.close();
  });
}

// ---------- 2. Barra, vista previa, Ajustes y diálogos ----------
async function main() {
  await withFreshData({ SIFT_DEV_DEMO: "1" }, shoot);
}

async function shoot(app) {
  try {
    console.log("Arrancando la versión de desarrollo con los datos de demostración…");
    // `$`: la dirección exacta de la barra (la de Ajustes empieza igual).
    const bar = await connect("localhost:1420/$");
    await sleep(2500);

    await showBar(bar);
    await grab(bar, "barra-vacia.jpg");

    // Movimiento: la entrada de los paneles usa opacidad y transformación, con su token.
    const swap = await bar.evaluate(`(() => {
      const el = document.querySelector('.sift-swap');
      if (!el) return null;
      const s = getComputedStyle(el);
      return { name: s.animationName, duration: s.animationDuration };
    })()`);
    check("Los paneles de la barra entran con animación", swap && swap.name !== "none", JSON.stringify(swap));

    // La vista previa de un Espacio: sus acciones en orden.
    await showBar(bar);
    const space = await searchUntil(bar, "tienda", (r) => r.includes("Tienda de Lucía"), 60_000);
    check("Buscar «tienda» trae el Espacio", space.includes("Tienda de Lucía"), space.slice(0, 120));
    await bar.press("Tab");
    const steps = await waitFor(
      () => bar.evaluate("document.querySelector('ol[aria-label]')?.innerText ?? ''"),
      5_000,
      "las acciones del Espacio",
    ).catch(() => "");
    check("Tab sobre un Espacio enseña sus acciones", /Colocar en/.test(steps), steps.slice(0, 120));
    await sleep(400);
    await grab(bar, "vista-previa-espacio.jpg");
    await bar.press("Escape");
    await sleep(300);

    // Búsqueda: esperar a que el índice de los datos de demostración esté listo.
    await showBar(bar);
    // Sift se pausa si el PC está ocupado: hay margen de sobra.
    const found = await searchUntil(bar, "pago", (r) => r.includes("contrato-servicios"), 300_000);
    check("Buscar «pago» trae resultados", found.includes("contrato-servicios"), found.slice(0, 120));
    await sleep(500);
    await showBar(bar);
    await grab(bar, "barra-busqueda.jpg");

    // Vista previa: Tab de verdad.
    await showBar(bar);
    await bar.press("Tab");
    const region = await waitFor(
      () => bar.evaluate("document.querySelector('[role=region][aria-label^=\"Vista previa\"]')?.innerText ?? ''"),
      5_000,
      "la vista previa",
    ).catch(() => "");
    check("Tab abre la vista previa", region.length > 0, region.slice(0, 120));
    check("La vista previa dice por qué sale", /Contiene|Trata de lo mismo|Por el nombre/.test(region));
    await sleep(400);
    await grab(bar, "vista-previa.jpg");
    const before = await bar.evaluate("document.querySelector('[role=region][aria-label^=\"Vista previa\"]')?.getAttribute('aria-label')");
    await bar.press("ArrowDown");
    await sleep(400);
    const after = await bar.evaluate("document.querySelector('[role=region][aria-label^=\"Vista previa\"]')?.getAttribute('aria-label')");
    check("↓ cambia de resultado en la vista previa", before && after && before !== after, `${before} → ${after}`);
    await bar.press("Escape");
    await sleep(300);
    const back = await bar.evaluate("document.querySelectorAll('[role=option]').length > 0 && document.visibilityState === 'visible'");
    check("Esc vuelve a la lista sin cerrar la barra", back);

    // Filtro rápido: «pdf » se convierte en etiqueta.
    await typeInBar(bar, "pdf ");
    await sleep(300);
    const chip = await bar.evaluate("document.querySelector('button[title^=\"Quitar el filtro\"]')?.textContent ?? ''");
    check("«pdf » pone la etiqueta del filtro", chip.includes("PDF") || chip.toLowerCase().includes("pdf"), chip);
    await typeInBar(bar, "");

    // Ajustes.
    await bar.evaluate("window.__TAURI_INTERNALS__.invoke('open_settings', { newSpace: false })");
    const settings = await connect("settings.html");
    await waitFor(() => text(settings, "h1"), 20_000, "Ajustes");
    await sleep(800);
    await grab(settings, "ajustes-general.jpg");

    const goTo = (label) =>
      settings.evaluate(`[...document.querySelectorAll('nav button')].find(b => b.textContent.trim() === ${JSON.stringify(label)})?.click()`);
    await goTo("Carpetas");
    await sleep(800);
    await grab(settings, "ajustes-carpetas.jpg");

    await goTo("Espacios");
    await sleep(800);
    await settings.evaluate(`document.querySelector('ul[aria-label="Espacios"] li button')?.click()`);
    // Que se vea la acción con su zona y su pantalla, no solo la cabecera.
    await sleep(800);
    await grab(settings, "ajustes-espacio.jpg");

    await goTo("Estado");
    // Antes de la prueba del diálogo del icono: recorrerlo con el teclado deja pegado un
    // globo de ayuda de Windows («Cambiar icono») encima de las capturas siguientes.
    parkCursor();
    await sleep(800);
    await grab(settings, "ajustes-estado.jpg");

    await goTo("Espacios");
    await sleep(800);
    // El diálogo del icono: atrapa el foco y lo devuelve al cerrar.
    await settings.evaluate(`document.querySelector('button[aria-label="Cambiar icono"]')?.focus()`);
    await settings.press("Enter");
    await sleep(500);
    const inDialog = () => settings.evaluate("!!document.activeElement?.closest('[role=dialog]')");
    check("El diálogo recibe el foco al abrirse", await inDialog());
    let stayed = true;
    for (let i = 0; i < 25; i++) {
      await settings.press("Tab");
      if (!(await inDialog())) stayed = false;
    }
    for (let i = 0; i < 5; i++) {
      await settings.press("Tab", { shift: true });
      if (!(await inDialog())) stayed = false;
    }
    check("Tab y Mayús+Tab no salen del diálogo", stayed);
    await grab(settings, "dialogo-icono.jpg");
    await settings.press("Escape");
    await sleep(400);
    const returned = await settings.evaluate("document.activeElement?.getAttribute('aria-label')");
    check("Al cerrar, el foco vuelve a quien abrió el diálogo", returned === "Cambiar icono", String(returned));


    const errors = [...bar.errors, ...settings.errors];
    check("Sin errores de JavaScript", errors.length === 0, errors.join(" ; ").slice(0, 200));
    bar.close();
    settings.close();
  } catch (e) {
    check("La prueba llegó al final", false, String(e));
    fs.writeFileSync(path.join(root, "target", "capturas-final-app.log"), app.log);
  }
}

try {
  if (!process.argv.includes("--sin-bienvenida")) await welcome();
  await main();
} catch (e) {
  check("La prueba llegó al final", false, String(e));
}
console.log(`\nCapturas en ${out}`);
process.exit(results.some((r) => !r.ok) ? 1 : 0);
