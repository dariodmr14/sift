// Prueba de carga: indexa una carpeta grande generada (generar.py) con la compilación de
// pruebas y mide tiempo de indexado, latencia de búsqueda (p50/p95), memoria, tamaño en disco
// y, con la búsqueda inteligente, el ritmo de vectorización y la búsqueda mientras tanto.
//
// Uso (desde la raíz; ver comun.mjs para compilar):
//   node scripts/aguante/carga.mjs "<carpeta de prueba>" <salida.json> [minutos de vectores]

import fs from "node:fs";
import path from "node:path";
import {
  DEV_DATA, connectBar, folderMb, invoke, memory, percentile, setAsideDevData, sleep, startSift, waitIndexed,
} from "./comun.mjs";

const [folder, outFile, vecMinArg] = process.argv.slice(2);
if (!folder || !outFile) throw new Error("uso: carga.mjs <carpeta> <salida.json> [minutos]");
const VECTOR_MINUTES = Number(vecMinArg ?? 20);
const log = (...a) => console.log(new Date().toISOString().slice(11, 19), ...a);

const QUERIES = {
  "1 letra": "p",
  "2 letras": "pr",
  "3 letras": "pre",
  "8 letras": "presupue",
  "palabra común": "proyecto",
  "palabra rara": "zarzamora",
  "con errata": "presupuseto",
  "frase": "reunión con el cliente",
  "filtro pdf": "pdf contrato",
  "código": "calcular_500",
};

let seq = 0;
async function measureSearch(bar, rounds) {
  const byKind = {};
  for (let r = 0; r < rounds; r++) {
    for (const [kind, q] of Object.entries(QUERIES)) {
      const t0 = performance.now();
      const res = await invoke(bar, "search_files", { query: q, seq: ++seq, filter: null });
      const ms = performance.now() - t0;
      (byKind[kind] ??= { ms: [], hits: 0 }).ms.push(ms);
      byKind[kind].hits = res?.results?.hits?.length ?? 0;
    }
  }
  const out = {};
  for (const [k, v] of Object.entries(byKind)) {
    out[k] = { p50: percentile(v.ms, 50), p95: percentile(v.ms, 95), max: percentile(v.ms, 100), hits: v.hits };
  }
  const all = Object.values(byKind).flatMap((v) => v.ms);
  return { all: { p50: percentile(all, 50), p95: percentile(all, 95), n: all.length }, byKind: out };
}

const restore = setAsideDevData();
const result = { folder, startedAt: new Date().toISOString() };
let app;
try {
  // 1. Indexado desde cero (sin búsqueda inteligente: sin su permiso no se descarga nada).
  app = startSift(folder);
  const bar = await connectBar();
  log("conectado; indexando…");
  const idx = await waitIndexed(bar, app.pid, { log });
  result.indexing = { seconds: idx.seconds, peakMemory: idx.peak, samples: idx.samples };
  const st = await invoke(bar, "index_status");
  result.indexing.files = st.total;
  log(`indexado: ${st.total} archivos en ${idx.seconds} s`);

  // 2. Reposo y búsqueda (en frío y en caliente).
  await sleep(20_000);
  result.idleMemory = memory(app.pid);
  result.search = await measureSearch(bar, 25);
  result.search.rust = (await invoke(bar, "get_status")).searchLatency ?? null;
  log("búsqueda:", JSON.stringify(result.search.all), "rust:", JSON.stringify(result.search.rust));
  result.disk = { totalMb: folderMb(DEV_DATA) };
  for (const e of fs.readdirSync(DEV_DATA, { withFileTypes: true })) {
    result.disk[e.name] = e.isDirectory() ? folderMb(path.join(DEV_DATA, e.name)) : Math.round(fs.statSync(path.join(DEV_DATA, e.name)).size / 2 ** 20 * 10) / 10;
  }
  log("disco:", JSON.stringify(result.disk));

  // 3. Búsqueda inteligente: descarga del modelo (hash fijado) y vectorización un rato.
  if (VECTOR_MINUTES > 0) {
    await invoke(bar, "allow_model_download");
    const vec = { samples: [], searchDuring: [] };
    const t0 = Date.now();
    while (Date.now() - t0 < VECTOR_MINUTES * 60_000) {
      await sleep(30_000);
      const s = (await invoke(bar, "index_status")).smart;
      const m = memory(app.pid);
      vec.samples.push({ t: Math.round((Date.now() - t0) / 1000), done: s?.done, total: s?.total, ready: s?.ready, mb: s?.downloadedMb, ...m });
      log(`vectores ${s?.done}/${s?.total} · ${m.siftWorkingMb} MB`);
      if (s?.ready && !s?.awaitingPermission && s.done > 0) {
        const r = await measureSearch(bar, 2);
        vec.searchDuring.push({ t: Math.round((Date.now() - t0) / 1000), ...r.all });
      }
    }
    result.vectors = vec;
  }

  // 4. Reinicio con todo indexado: cuánto tarda en estar listo y la primera búsqueda.
  app.stop();
  await sleep(4000);
  const t0 = Date.now();
  app = startSift(null);
  const bar2 = await connectBar();
  await sleep(1000);
  const first = performance.now();
  await invoke(bar2, "search_files", { query: "proyecto", seq: ++seq, filter: null });
  result.restart = { secondsToBar: Math.round((Date.now() - t0) / 100) / 10, firstSearchMs: Math.round(performance.now() - first) };
  // Un minuto buscando cada 250 ms mientras arranca y vectoriza (¿espera la barra a algo?).
  const probe = [];
  const probeStart = Date.now();
  while (Date.now() - probeStart < 60_000) {
    const t = performance.now();
    await invoke(bar2, "search_files", { query: "pr", seq: ++seq, filter: null });
    probe.push(performance.now() - t);
    await sleep(250);
  }
  result.restart.probe = { n: probe.length, p50: percentile(probe, 50), p95: percentile(probe, 95), max: percentile(probe, 100), over100ms: probe.filter((ms) => ms > 100).length };
  log("sonda tras reiniciar:", JSON.stringify(result.restart.probe));
  result.restart.after15s = await measureSearch(bar2, 5);
  result.restart.memory = memory(app.pid);
  log("reinicio:", JSON.stringify(result.restart));
} catch (e) {
  result.error = String(e?.stack ?? e);
  log("ERROR", result.error);
} finally {
  app?.stop();
  await sleep(4000);
  result.devDataRestored = restore();
  result.finishedAt = new Date().toISOString();
  fs.writeFileSync(outFile, JSON.stringify(result, null, 2));
  log("resultado en", outFile, "· datos de desarrollo devueltos:", result.devDataRestored);
}
