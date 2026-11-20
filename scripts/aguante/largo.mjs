// Prueba de larga duración: la compilación de pruebas abierta durante horas con actividad
// simulada (búsquedas escritas en la barra, archivos que se crean, cambian y borran, y un
// Espacio que se abre), midiendo la memoria cada minuto para ver si crece sin parar.
//
// Uso (desde la raíz, con la carpeta ya indexada o no):
//   node scripts/aguante/largo.mjs "<carpeta de prueba>" <salida.json> <horas>

import fs from "node:fs";
import path from "node:path";
import { typeInBar } from "../devapp.mjs";
import { connectBar, invoke, memory, setAsideDevData, sleep, startSift, waitIndexed } from "./comun.mjs";

const [folder, outFile, hoursArg] = process.argv.slice(2);
if (!folder || !outFile) throw new Error("uso: largo.mjs <carpeta> <salida.json> <horas>");
const HOURS = Number(hoursArg ?? 3);
const log = (...a) => console.log(new Date().toISOString().slice(11, 19), ...a);
const QUERIES = ["proyecto", "factura", "reunión cliente", "pdf contrato", "presupuesto", "receta", "calcular", "tren", "examen", "zarzamora", "notas", "red protocolo"];
const changes = path.join(folder, "cambios-largo");

/** Escribe, cambia o borra archivos de prueba (que el vigilante tiene que ver). */
let n = 0;
function churn() {
  fs.mkdirSync(changes, { recursive: true });
  const now = fs.readdirSync(changes);
  n++;
  fs.writeFileSync(path.join(changes, `nota-larga-${n}.md`), `# Nota ${n}\n\nProyecto de prueba ${n}: reunión, factura y presupuesto.\n`);
  if (now.length > 3) fs.appendFileSync(path.join(changes, now[n % now.length]), `\nCambio ${n}.\n`);
  if (now.length > 40) fs.rmSync(path.join(changes, now[0]), { force: true });
}

const restore = setAsideDevData();
const result = { folder, hours: HOURS, startedAt: new Date().toISOString(), samples: [], errors: [] };
let app;
try {
  app = startSift(folder);
  const bar = await connectBar();
  log("indexando primero…");
  result.indexing = (await waitIndexed(bar, app.pid, { log })).seconds;
  const space = await invoke(bar, "create_space", {
    draft: { name: "Prueba de aguante", aliases: [], icon: { kind: "none" }, actions: [{ kind: "wait", until: { kind: "seconds", secs: 1 }, timeoutSecs: 10, onTimeout: "stop" }] },
  });
  const t0 = Date.now();
  let lastSample = 0, lastChurn = 0, lastSpace = 0, q = 0, searches = 0, spaces = 0;
  while (Date.now() - t0 < HOURS * 3600_000) {
    const t = Date.now() - t0;
    try {
      await typeInBar(bar, QUERIES[q++ % QUERIES.length]);
      searches++;
      if (t - lastChurn > 20_000) { churn(); lastChurn = t; }
      if (t - lastSpace > 60_000) {
        const r = await invoke(bar, "run_space", { id: space.id });
        if (r?.status !== "done") result.errors.push({ t, space: r });
        spaces++;
        lastSpace = t;
      }
      if (t - lastSample > 60_000) {
        const m = memory(app.pid);
        const s = await invoke(bar, "index_status");
        const st = await invoke(bar, "get_status");
        result.samples.push({ min: Math.round(t / 60000), ...m, searches, spaces, indexing: s.running, p95: st.searchLatency?.p95Ms ?? null });
        log(`${Math.round(t / 60000)} min · Sift ${m.siftWorkingMb} MB · WebView ${m.webviewMb} MB · ${searches} búsquedas · p95 ${st.searchLatency?.p95Ms ?? "–"} ms`);
        lastSample = t;
      }
    } catch (e) {
      result.errors.push({ t, error: String(e) });
      log("error:", String(e));
    }
    await sleep(3000);
  }
  await typeInBar(bar, "");
  // Pendiente de la memoria (MB por hora), sin los 10 primeros minutos (calentamiento).
  const fit = (key) => {
    const pts = result.samples.filter((s) => s.min >= 10).map((s) => [s.min / 60, s[key]]);
    if (pts.length < 3) return null;
    const mx = pts.reduce((a, p) => a + p[0], 0) / pts.length, my = pts.reduce((a, p) => a + p[1], 0) / pts.length;
    const num = pts.reduce((a, p) => a + (p[0] - mx) * (p[1] - my), 0), den = pts.reduce((a, p) => a + (p[0] - mx) ** 2, 0);
    return Math.round((num / den) * 10) / 10;
  };
  result.slopeMbPerHour = { sift: fit("siftWorkingMb"), siftPrivate: fit("siftPrivateMb"), webview: fit("webviewMb") };
  log("pendiente (MB/hora):", JSON.stringify(result.slopeMbPerHour));
} catch (e) {
  result.fatal = String(e?.stack ?? e);
  log("ERROR", result.fatal);
} finally {
  app?.stop();
  await sleep(4000);
  fs.rmSync(changes, { recursive: true, force: true });
  result.devDataRestored = restore();
  result.finishedAt = new Date().toISOString();
  fs.writeFileSync(outFile, JSON.stringify(result, null, 2));
  log("resultado en", outFile, "· datos devueltos:", result.devDataRestored);
}
