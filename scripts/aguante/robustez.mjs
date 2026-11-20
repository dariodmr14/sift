// Pruebas de robustez con la compilación de pruebas:
//  1. Cierres bruscos a mitad de indexado (como un corte de luz): al volver a abrir, sigue
//     y acaba sin errores, y la búsqueda funciona.
//  2. Archivos raros: enorme, corruptos, binarios con extensión de texto, vacíos, rutas de más
//     de 260 caracteres, nombres con símbolos y una carpeta sin permiso de lectura.
// (El disco lleno se prueba en Rust: crates/storage, test `disk_full_*`.)
//
// Uso (desde la raíz):
//   node scripts/aguante/robustez.mjs "<carpeta grande>" "<carpeta de archivos raros>" <salida.json>

import { execFileSync } from "node:child_process";
import crypto from "node:crypto";
import fs from "node:fs";
import path from "node:path";
import { connectBar, invoke, memory, setAsideDevData, sleep, startSift, waitIndexed } from "./comun.mjs";

const [big, odd, outFile] = process.argv.slice(2);
if (!big || !odd || !outFile) throw new Error("uso: robustez.mjs <carpeta grande> <carpeta rara> <salida.json>");
const log = (...a) => console.log(new Date().toISOString().slice(11, 19), ...a);
const result = { startedAt: new Date().toISOString() };
const user = process.env.USERNAME;

/** La carpeta de archivos raros (todo inventado). */
function makeOdd() {
  fs.rmSync(odd, { recursive: true, force: true });
  fs.mkdirSync(odd, { recursive: true });
  // Enorme: 600 MB de texto.
  const line = "linea de texto de un archivo enorme con palabras como presupuesto y proyecto\n";
  const chunk = line.repeat(Math.ceil((8 << 20) / line.length));
  const fd = fs.openSync(path.join(odd, "enorme.txt"), "w");
  for (let i = 0; i < 75; i++) fs.writeSync(fd, chunk);
  fs.closeSync(fd);
  // Corruptos y truncados.
  fs.writeFileSync(path.join(odd, "corrupto.pdf"), crypto.randomBytes(200_000));
  const pdf = fs.readFileSync(findPdf(big));
  fs.writeFileSync(path.join(odd, "truncado.pdf"), pdf.subarray(0, Math.floor(pdf.length / 2)));
  fs.writeFileSync(path.join(odd, "falso.docx"), crypto.randomBytes(100_000));
  fs.writeFileSync(path.join(odd, "falso.xlsx"), Buffer.from("PK\u0003\u0004 esto no es un zip de verdad"));
  fs.writeFileSync(path.join(odd, "binario.txt"), crypto.randomBytes(2_000_000));
  fs.writeFileSync(path.join(odd, "vacio.md"), "");
  // Nombres raros.
  fs.writeFileSync(path.join(odd, "ñandú café — notas (copia) #1 😀.md"), "Notas con un nombre raro: presupuesto.");
  // Ruta de más de 260 caracteres.
  let deep = odd;
  while (deep.length < 300) deep = path.join(deep, "carpeta-con-un-nombre-largo");
  fs.mkdirSync("\\\\?\\" + deep, { recursive: true });
  fs.writeFileSync("\\\\?\\" + path.join(deep, "nota-profunda.md"), "Una nota muy profunda: presupuesto.");
  // Sin permiso de lectura para quien ejecuta la prueba (se quita al acabar).
  const locked = path.join(odd, "sin-permiso");
  fs.mkdirSync(locked);
  fs.writeFileSync(path.join(locked, "secreto.md"), "No se puede leer.");
  execFileSync("icacls", [locked, "/deny", `${user}:(OI)(CI)(R)`], { stdio: "ignore" });
  return locked;
}

function findPdf(dir) {
  for (const e of fs.readdirSync(dir, { withFileTypes: true })) {
    const p = path.join(dir, e.name);
    if (e.isDirectory()) {
      const f = findPdf(p);
      if (f) return f;
    } else if (e.name.endsWith(".pdf")) return p;
  }
  return null;
}

let restore = setAsideDevData();
let app, locked;
try {
  // 1. Cierres bruscos a mitad de indexado.
  const crashes = [];
  for (const after of [25_000, 60_000]) {
    app = startSift(big);
    const bar = await connectBar();
    await sleep(after);
    const s = await invoke(bar, "index_status");
    crashes.push({ killedAfterMs: after, done: s.done, total: s.total, phase: s.phase });
    log(`cierre brusco tras ${after / 1000} s (${s.done}/${s.total})`);
    app.kill();
    await sleep(3000);
  }
  app = startSift(null);
  const bar = await connectBar();
  const idx = await waitIndexed(bar, app.pid, { log });
  const s = await invoke(bar, "index_status");
  const q = await invoke(bar, "search_files", { query: "presupuesto", seq: 1, filter: null });
  result.crash = { crashes, recoveredSeconds: idx.seconds, files: s.total, error: s.error ?? null, searchHits: q?.results?.hits?.length ?? 0 };
  log("recuperado:", JSON.stringify(result.crash));
  app.stop();
  await sleep(4000);
  restore();
  restore = null;

  // 2. Archivos raros (en una carpeta aparte, desde cero).
  const restore2 = setAsideDevData();
  try {
    locked = makeOdd();
    app = startSift(odd);
    const bar2 = await connectBar();
    const idx2 = await waitIndexed(bar2, app.pid, { log, every: 2000 });
    const st = await invoke(bar2, "index_status");
    const roots = await invoke(bar2, "list_roots");
    const unread = roots.length ? await invoke(bar2, "unread_files", { rootId: roots[0].id }) : [];
    const hits = await invoke(bar2, "search_files", { query: "presupuesto", seq: 2, filter: null });
    const deepHit = await invoke(bar2, "search_files", { query: "profunda", seq: 3, filter: null });
    result.odd = {
      seconds: idx2.seconds, peakMemory: idx2.peak, files: st.total, error: st.error ?? null,
      unread, searchHits: hits?.results?.hits?.map((h) => h.name) ?? [],
      deepFound: (deepHit?.results?.hits ?? []).some((h) => h.name === "nota-profunda.md"),
      memoryAfter: memory(app.pid),
    };
    log("archivos raros:", JSON.stringify(result.odd));
  } finally {
    app?.stop();
    await sleep(4000);
    result.devDataRestored = restore2();
  }
} catch (e) {
  result.error = String(e?.stack ?? e);
  log("ERROR", result.error);
  app?.kill();
  await sleep(3000);
} finally {
  if (restore) result.devDataRestored = restore();
  if (locked) execFileSync("icacls", [locked, "/remove:d", user], { stdio: "ignore" });
  fs.rmSync("\\\\?\\" + odd, { recursive: true, force: true });
  result.finishedAt = new Date().toISOString();
  fs.writeFileSync(outFile, JSON.stringify(result, null, 2));
  log("resultado en", outFile);
}
