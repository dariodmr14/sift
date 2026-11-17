// Capturas de antes y después de cada pasada de unslop (docs/capturas/).
//
// Uso (desde la raíz del repo, en la rama de las pasadas, sin cambios sin guardar en
// apps/desktop/src; abre ventanas en la pantalla y no hay que tocar nada mientras tanto):
//   node scripts/capturas.mjs [rama-base]        (por defecto, main)
//
// Arranca UNA vez la versión de DESARROLLO (sus datos de prueba, nunca los tuyos) y, para
// cada estado (la base y cada commit «(unslop)» de la rama), pone `apps/desktop/src` como
// estaba en ese commit: Vite recarga la interfaz sola. El Rust es el de la rama, que sirve
// para todos los estados. Primero la barra en todos los estados y después Ajustes, porque
// al abrir Ajustes la barra se oculta. Al acabar, deja `apps/desktop/src` como estaba.
//
// La captura es de la pantalla de verdad (scripts/captura-region.ps1): el vidrio lo pinta
// Windows y el WebView no lo ve.

import { execFileSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";
import { connect, root, sleep, startDevApp, typeInBar, waitFor } from "./devapp.mjs";

const base = process.argv[2] ?? "main";
const out = path.join(root, "docs", "capturas");
const git = (...args) => execFileSync("git", args, { cwd: root, encoding: "utf8" }).trim();

if (git("status", "--porcelain", "--", "apps/desktop/src")) {
  console.error("Hay cambios sin guardar en apps/desktop/src: guárdalos antes.");
  process.exit(1);
}

// Los estados: la base y cada pasada, en orden.
const passes = git("log", "--reverse", "--format=%H%x09%s", `${base}..HEAD`)
  .split("\n")
  .filter((l) => l.includes("(unslop):"))
  .map((l) => {
    const [sha, subject] = l.split("\t");
    const name = subject.match(/—\s*(.*)$/)?.[1] ?? subject;
    const pass = subject.match(/\(unslop\): (\w+) pass/)?.[1] ?? "pasada";
    return { sha, slug: pass, name };
  });
const states = [
  { sha: git("merge-base", base, "HEAD"), slug: "antes", name: "antes de unslop" },
  ...passes,
];
console.log(states.map((s, i) => `${i}. ${s.slug} (${s.sha.slice(0, 7)})`).join("\n"));

function useSrcOf(sha) {
  execFileSync("git", ["checkout", sha, "--", "apps/desktop/src"], { cwd: root });
}

/** Captura la ventana de la página tal como se ve en pantalla. */
async function grab(page, file) {
  const r = await page.evaluate(`({
    x: Math.round(window.screenX * devicePixelRatio),
    y: Math.round(window.screenY * devicePixelRatio),
    w: Math.round(window.outerWidth * devicePixelRatio),
    h: Math.round(window.outerHeight * devicePixelRatio),
  })`);
  fs.mkdirSync(path.dirname(file), { recursive: true });
  execFileSync("powershell", [
    "-NoProfile",
    "-ExecutionPolicy",
    "Bypass",
    "-File",
    path.join(root, "scripts", "captura-region.ps1"),
    "-X",
    String(r.x),
    "-Y",
    String(r.y),
    "-W",
    String(r.w),
    "-H",
    String(r.h),
    "-Out",
    file,
  ]);
}

/** Recarga la página y espera a que `selector` exista (la nueva interfaz ya está). */
async function reload(page, selector) {
  await page.evaluate("location.reload()").catch(() => {});
  await sleep(1500);
  await waitFor(
    () => page.evaluate(`!!document.querySelector(${JSON.stringify(selector)})`).catch(() => false),
    30_000,
    selector,
  );
  await sleep(800); // iconos y resultados
}

const app = startDevApp();
let code = 0;
try {
  console.log("Arrancando la versión de desarrollo…");
  const bar = await connect("localhost:1420/");
  await sleep(2000);

  for (const [i, s] of states.entries()) {
    useSrcOf(s.sha);
    await reload(bar, 'input[aria-label="Buscar"]');
    const prefix = path.join(out, `${i}-${s.slug}`);
    await grab(bar, `${prefix}-barra-vacia.jpg`);
    await typeInBar(bar, "vidrio");
    await waitFor(() => bar.evaluate("document.querySelectorAll('[role=option]').length > 0"), 20_000, "resultados").catch(() => {});
    await sleep(600);
    await grab(bar, `${prefix}-barra-busqueda.jpg`);
    console.log(`barra: ${s.slug}`);
  }

  await bar.evaluate("window.__TAURI_INTERNALS__.invoke('open_settings', { newSpace: false })");
  const settings = await connect("settings.html");
  for (const [i, s] of states.entries()) {
    useSrcOf(s.sha);
    await reload(settings, "h1");
    const prefix = path.join(out, `${i}-${s.slug}`);
    await grab(settings, `${prefix}-ajustes-general.jpg`);
    // Espacios: el primero, abierto en el editor.
    await settings.evaluate(`(() => {
      const nav = [...document.querySelectorAll('nav button')].find(b => b.textContent.trim() === 'Espacios');
      nav?.click();
    })()`);
    await sleep(800);
    await settings.evaluate(`document.querySelector('ul[aria-label="Espacios"] li button')?.click()`);
    await sleep(800);
    await grab(settings, `${prefix}-ajustes-espacio.jpg`);
    console.log(`ajustes: ${s.slug}`);
  }
  bar.close();
  settings.close();
} catch (e) {
  console.error(String(e));
  fs.writeFileSync(path.join(root, "target", "capturas-app.log"), app.log);
  code = 1;
} finally {
  // `apps/desktop/src` como en la rama, sin dejar archivos de estados anteriores.
  useSrcOf("HEAD");
  app.stop();
  console.log(`\nCapturas en ${out}`);
  process.exit(code);
}
