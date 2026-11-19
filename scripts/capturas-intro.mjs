// Recorre la introducción guiada en la versión de DESARROLLO y hace una captura de cada
// paso (docs/capturas/introduccion/). Abre ventanas y pulsa el atajo de desarrollo: no
// hay que tocar nada mientras tanto.
//
// Uso (desde la raíz del repo):
//   node scripts/capturas-intro.mjs
//
// Igual que la bienvenida de scripts/capturas-final.mjs: aparta la carpeta de datos de
// desarrollo (%APPDATA%\dev.sift.app-dev), arranca sin datos de prueba
// (SIFT_DEV_NO_SEED=1) y con los datos de demostración (SIFT_DEV_DEMO=1: la carpeta
// sugerida es la copia de demo/ en Documentos públicos, no tus Documentos),
// recorre los cinco pasos, cierra, borra la carpeta nueva y devuelve la de antes. No
// descarga el modelo («Ahora no»). Sale con 1 si alguna comprobación falla.

import { execFileSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";
import { connect, root, sleep, startDevApp, waitFor } from "./devapp.mjs";

const out = path.join(root, "docs", "capturas", "introduccion");
fs.mkdirSync(out, { recursive: true });
const devData = path.join(process.env.APPDATA, "dev.sift.app-dev");

const results = [];
const check = (name, ok, detail = "") => {
  results.push({ name, ok: Boolean(ok), detail });
  console.log(`${ok ? "OK   " : "FALLA"} ${name}${detail ? ` · ${detail}` : ""}`);
};

/** Captura la ventana tal como se ve en pantalla (el vidrio lo pinta Windows). */
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

const bodyText = (page) => page.evaluate("document.body.innerText");
const heading = (page) =>
  page.evaluate("document.querySelector('h1')?.textContent ?? null");

/** Pulsa el botón con ese texto, como un clic. */
async function click(page, label) {
  const ok = await page.evaluate(`(() => {
    const b = [...document.querySelectorAll('button')].find(
      (x) => x.textContent.trim() === ${JSON.stringify(label)} && !x.disabled);
    b?.click();
    return Boolean(b);
  })()`);
  if (!ok) throw new Error(`no encuentro el botón «${label}»`);
}

/** Escribe en un campo (por su etiqueta accesible) como una persona: dispara `input`. */
function type(page, label, text) {
  return page.evaluate(`(() => {
    const box = [...document.querySelectorAll('input')].find(
      (x) => x.getAttribute('aria-label') === ${JSON.stringify(label)} ||
        x.closest('label')?.textContent.trim().startsWith(${JSON.stringify(label)}));
    const set = Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, 'value').set;
    set.call(box, ${JSON.stringify(text)});
    box.dispatchEvent(new Event('input', { bubbles: true }));
  })()`);
}

/** Espera a que un texto salga en la página. */
const until = (page, re, ms, what) =>
  waitFor(async () => re.test(await bodyText(page)), ms, what);

async function walk(page) {
  // 1. Tu atajo: se pulsa de verdad (el de desarrollo, Ctrl+Alt+Espacio).
  const h1 = await waitFor(() => heading(page), 60_000, "la introducción");
  check("La introducción sale en un equipo nuevo", h1 === "Te damos la bienvenida a Sift", h1);
  await sleep(800);
  await grab(page, "1-atajo.jpg");
  execFileSync("powershell", [
    "-NoProfile", "-ExecutionPolicy", "Bypass",
    "-File", path.join(root, "scripts", "atajo-dev.ps1"),
  ]);
  const bar = await connect("localhost:1420/$");
  await waitFor(
    () => bar.evaluate("document.visibilityState === 'visible'"),
    10_000,
    "la barra a la vista",
  );
  await sleep(600);
  await bar.press("Escape");
  // WebView2 no cambia `visibilityState` al ocultar la ventana: la señal fiable es que la
  // barra recibe el aviso de Rust y vuelve a su punto de partida (`sift-from`).
  const closed = await waitFor(
    () => bar.evaluate("document.querySelector('main')?.classList.contains('sift-from')"),
    5_000,
    "la barra cerrada con Esc",
  ).catch(() => false);
  check("La barra se cierra con Esc", closed);
  const detected = await until(page, /Eso es/, 10_000, "el atajo detectado").catch(() => false);
  check("Detecta el atajo pulsado de verdad", detected);
  await sleep(500);
  await grab(page, "1-atajo-hecho.jpg");
  bar.close();

  // 2. Dónde buscar.
  await click(page, "Siguiente");
  await sleep(500);
  check("Paso 2: Dónde buscar", (await heading(page)) === "Dónde buscar");
  await click(page, "Añadir Documentos");
  const added = await until(page, /Documentos añadida/, 15_000, "la carpeta añadida").catch(() => false);
  check("Añade la carpeta sugerida", added);
  const indexed = await until(page, /Al día/, 120_000, "la carpeta leída").catch(() => false);
  check("La carpeta se lee hasta estar al día", indexed);
  await sleep(500);
  await grab(page, "2-carpetas.jpg");

  // 3. Prueba a buscar: resultados de verdad del índice.
  await click(page, "Siguiente");
  await sleep(500);
  await type(page, "Buscar", "pago");
  const found = await waitFor(
    () => page.evaluate("document.querySelectorAll('[aria-label=Resultados] button').length"),
    15_000,
    "resultados de la búsqueda",
  ).catch(() => 0);
  check("La búsqueda de prueba enseña resultados reales", found > 0, `${found} resultados`);
  await sleep(500);
  await grab(page, "3-buscar.jpg");

  // 4. Búsqueda inteligente: se ve el permiso; no se descarga.
  await click(page, "Siguiente");
  await sleep(500);
  check("Pide permiso antes de descargar", /Descargar \(\d+ MB\)/.test(await bodyText(page)));
  await grab(page, "4-inteligente.jpg");
  await click(page, "Ahora no");

  // 5. Tu primer Espacio.
  await click(page, "Siguiente");
  await sleep(500);
  await type(page, "Nombre", "Tienda de Lucía");
  await type(page, "Una web que abre (opcional)", "example.com");
  await sleep(300);
  await grab(page, "5-espacio.jpg");
  await click(page, "Crear el Espacio");
  const created = await until(page, /creado/, 10_000, "el Espacio creado").catch(() => false);
  check("Crea el primer Espacio", created);
  await sleep(400);
  await grab(page, "5-espacio-hecho.jpg");

  // Final: «Pulsa… para empezar» y se abre la barra.
  await click(page, "Empezar");
  await sleep(300);
  await grab(page, "6-final.jpg");
  check("Sin errores de JavaScript en la introducción", page.errors.length === 0, page.errors.join(" ; ").slice(0, 200));
}

async function main() {
  const backup = `${devData}.antes-capturas`;
  if (fs.existsSync(backup)) throw new Error(`ya existe ${backup}: revísalo a mano antes de seguir`);
  const had = fs.existsSync(devData);
  if (had) fs.renameSync(devData, backup);
  const app = startDevApp({ SIFT_DEV_NO_SEED: "1", SIFT_DEV_DEMO: "1" });
  try {
    console.log("Arrancando la versión de desarrollo sin datos…");
    const page = await connect("settings.html");
    await walk(page);
    page.close();
  } catch (e) {
    check("El recorrido llega al final", false, String(e?.message ?? e));
  } finally {
    app.stop();
    await sleep(2500); // que suelte los archivos
    if (fs.existsSync(devData)) fs.rmSync(devData, { recursive: true, force: true });
    if (had) fs.renameSync(backup, devData);
  }
  check("La carpeta de datos de desarrollo queda como estaba", fs.existsSync(devData) === had);
  const failed = results.filter((r) => !r.ok);
  console.log(`\n${results.length - failed.length} de ${results.length} comprobaciones bien.`);
  process.exit(failed.length ? 1 : 0);
}

await main();
