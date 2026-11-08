import { useEffect, useState } from "react";
import { commands, type Status } from "../bindings";

const BACKEND_LABEL = {
  registerHotKey: "RegisterHotKey (vía principal)",
  keyboardHook: "Hook de teclado (vía alternativa)",
} as const;

/** Ventana de ajustes. De momento muestra el estado; los controles llegan después. */
function Settings() {
  const [status, setStatus] = useState<Status | null>(null);

  useEffect(() => {
    void commands.getStatus().then(setStatus);
  }, []);

  return (
    <main className="h-full overflow-y-auto bg-(--surface) p-6 select-text">
      <h1 className="mb-6 text-2xl font-semibold">Ajustes</h1>

      <section aria-labelledby="estado" className="space-y-2">
        <h2
          id="estado"
          className="text-sm font-semibold tracking-wide text-(--text-muted) uppercase"
        >
          Estado
        </h2>
        {status ? <StatusList status={status} /> : <p>Cargando…</p>}
      </section>
    </main>
  );
}

function StatusList({ status }: { status: Status }) {
  const l = status.latency;
  return (
    <dl className="grid grid-cols-[auto_1fr] gap-x-6 gap-y-1 text-sm">
      <dt className="text-(--text-muted)">Atajo</dt>
      <dd>{status.hotkey ?? "ninguno (no se pudo registrar)"}</dd>
      <dt className="text-(--text-muted)">Mecanismo</dt>
      <dd>{status.backend ? BACKEND_LABEL[status.backend] : "—"}</dd>
      <dt className="text-(--text-muted)">Acrylic</dt>
      <dd>{status.acrylic ? "activo" : "no disponible (fondo opaco)"}</dd>
      <dt className="text-(--text-muted)">Latencia</dt>
      <dd>
        {l
          ? `última ${l.lastMs} ms · p50 ${l.p50Ms} ms · p95 ${l.p95Ms} ms (${l.samples} medidas)`
          : "aún sin medidas: abre el lanzador con el atajo"}
      </dd>
    </dl>
  );
}

export default Settings;
