import { useCallback, useEffect, useState } from "react";
import { commands, events, type IndexStatus, type RootView } from "../bindings";
import { Button, ErrorText } from "./ui";

/** «hace 3 min», «hace 2 h»… a partir de una fecha ISO de SQLite. */
export function since(
  iso: string | null | undefined,
  now = Date.now(),
): string {
  if (!iso) return "aún no";
  const secs = Math.max(0, Math.round((now - Date.parse(iso)) / 1000));
  if (secs < 60) return "hace un momento";
  if (secs < 3600) return `hace ${Math.round(secs / 60)} min`;
  if (secs < 86400) return `hace ${Math.round(secs / 3600)} h`;
  return `hace ${Math.round(secs / 86400)} d`;
}

/** Ajustes → Carpetas: qué se indexa, qué se ignora y cómo va el indexado. */
export function FoldersSection() {
  const [roots, setRoots] = useState<RootView[] | null>(null);
  const [status, setStatus] = useState<IndexStatus | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [confirmRemove, setConfirmRemove] = useState<number | null>(null);

  const reloadRoots = useCallback(
    () =>
      commands.listRoots().then((r) => {
        if (r.status === "ok") setRoots(r.data);
        else setError(r.error.message);
      }),
    [],
  );

  useEffect(() => {
    void reloadRoots();
    void commands.indexStatus().then(setStatus);
    // El indexado avisa de su progreso; al terminar, se recargan los recuentos.
    const unlisten = events.indexProgress.listen((e) => {
      setStatus(e.payload);
      if (!e.payload.running) void reloadRoots();
    });
    return () => {
      void unlisten.then((stop) => stop());
    };
  }, [reloadRoots]);

  async function add() {
    const r = await commands.addRoot();
    if (r.status === "error") setError(r.error.message);
    else {
      setError(null);
      void reloadRoots();
    }
  }

  async function remove(id: number) {
    setConfirmRemove(null);
    const r = await commands.removeRoot(id);
    if (r.status === "error") setError(r.error.message);
    void reloadRoots();
  }

  return (
    <div className="h-full space-y-8 overflow-y-auto p-6">
      <section className="space-y-3">
        <header className="flex items-baseline justify-between gap-4">
          <h2 className="text-sm font-semibold tracking-wide text-muted uppercase">
            Carpetas indexadas
          </h2>
          <Button variant="primary" onClick={() => void add()}>
            + Añadir carpeta…
          </Button>
        </header>
        <p className="text-sm text-muted">
          Sift busca por nombre y por contenido en estas carpetas y en todo lo
          que tienen dentro. Nada sale de tu PC.
        </p>
        {error && <ErrorText>{error}</ErrorText>}
        <ul className="divide-y divide-line rounded-window border border-line">
          {roots?.length === 0 && (
            <li className="p-4 text-sm text-muted">
              Aún no hay carpetas. Añade, por ejemplo, la de tus apuntes de la
              universidad.
            </li>
          )}
          {roots?.map((root) => (
            <li key={root.id} className="flex items-center gap-3 px-4 py-3">
              <span className="min-w-0 flex-1">
                <span className="block truncate text-sm" title={root.path}>
                  {root.path}
                </span>
                <span className="block text-xs text-muted">
                  {root.missing ? (
                    <span className="text-danger">
                      No se encuentra la carpeta (¿disco desconectado?)
                    </span>
                  ) : (
                    <>
                      {root.files.toLocaleString("es")} archivos
                      {root.errors > 0 && ` · ${root.errors} sin poder leer`}
                      {" · indexada "}
                      {since(root.lastIndexedAt)}
                    </>
                  )}
                </span>
              </span>
              {confirmRemove === root.id ? (
                <span className="flex items-center gap-2 text-sm">
                  ¿Dejar de indexarla?
                  <Button variant="danger" onClick={() => void remove(root.id)}>
                    Sí, quitar
                  </Button>
                  <Button
                    variant="ghost"
                    onClick={() => setConfirmRemove(null)}
                  >
                    No
                  </Button>
                </span>
              ) : (
                <Button
                  variant="ghost"
                  onClick={() => setConfirmRemove(root.id)}
                  aria-label={`Quitar ${root.path}`}
                >
                  Quitar
                </Button>
              )}
            </li>
          ))}
        </ul>
        <p className="text-xs text-muted">
          Quitar una carpeta solo la saca del índice: tus archivos no se tocan.
        </p>
      </section>

      <StatusPanel status={status} />
      <IgnorePatterns />
    </div>
  );
}

function StatusPanel({ status }: { status: IndexStatus | null }) {
  const last = status?.last;
  return (
    <section className="space-y-3" aria-live="polite">
      <header className="flex items-baseline justify-between gap-4">
        <h2 className="text-sm font-semibold tracking-wide text-muted uppercase">
          Estado del indexado
        </h2>
        <Button
          disabled={status?.running}
          onClick={() => void commands.reindex()}
        >
          Reindexar ahora
        </Button>
      </header>
      {status?.running ? (
        <div className="space-y-2">
          <p className="text-sm">
            Indexando… {status.processed.toLocaleString("es")} archivos
            revisados
            {status.currentRoot && (
              <span className="text-muted"> · {status.currentRoot}</span>
            )}
          </p>
          {/* Barra indeterminada: el total no se sabe hasta terminar de recorrer. */}
          <div className="h-1 overflow-hidden rounded-item bg-hover">
            <div className="sift-progress h-full w-1/3 rounded-item bg-accent" />
          </div>
        </div>
      ) : last ? (
        <p className="text-sm text-muted">
          Última pasada en {(last.seconds ?? 0).toFixed(1)} s: {last.added}{" "}
          nuevos, {last.updated} actualizados, {last.unchanged} sin cambios,{" "}
          {last.removed} quitados
          {last.errors > 0 && `, ${last.errors} sin poder leer`}.
        </p>
      ) : (
        <p className="text-sm text-muted">Aún no se ha indexado nada.</p>
      )}
      {status?.error && <ErrorText>{status.error}</ErrorText>}
      <p className="text-xs text-muted">
        Sift revisa las carpetas al arrancar y cuando las cambias aquí; solo
        relee lo que ha cambiado. Por ahora el contenido se lee en texto,
        Markdown y código; PDF y Word se encuentran por nombre (el contenido
        llega en la Fase 3).
      </p>
    </section>
  );
}

function IgnorePatterns() {
  const [text, setText] = useState("");
  const [saved, setSaved] = useState("");
  const [message, setMessage] = useState<{ ok: boolean; text: string } | null>(
    null,
  );

  useEffect(() => {
    void commands.getIgnorePatterns().then((r) => {
      if (r.status === "ok") {
        setText(r.data.join("\n"));
        setSaved(r.data.join("\n"));
      }
    });
  }, []);

  async function save() {
    const r = await commands.setIgnorePatterns(text.split(/\r?\n/));
    if (r.status === "error")
      return setMessage({ ok: false, text: r.error.message });
    setText(r.data.join("\n"));
    setSaved(r.data.join("\n"));
    setMessage({ ok: true, text: "Guardado. El índice se está actualizando." });
  }

  return (
    <section className="space-y-3">
      <h2 className="text-sm font-semibold tracking-wide text-muted uppercase">
        Ignorar
      </h2>
      <label className="block space-y-1">
        <span className="text-xs text-muted">
          Uno por línea. Un nombre (
          <code className="font-mono">node_modules</code>,{" "}
          <code className="font-mono">*.tmp</code>) se ignora esté donde esté;
          con barra (<code className="font-mono">Antiguo/*</code>) se mira desde
          la carpeta indexada.
        </span>
        <textarea
          spellCheck={false}
          rows={8}
          value={text}
          onChange={(e) => setText(e.currentTarget.value)}
          className="w-full rounded-item border border-line bg-hover px-3 py-2 font-mono text-sm outline-none focus:border-accent"
        />
      </label>
      <div className="flex items-center gap-3">
        <Button
          variant="primary"
          disabled={text === saved}
          onClick={() => void save()}
        >
          Guardar patrones
        </Button>
        {message &&
          (message.ok ? (
            <span role="status" className="text-sm text-success">
              {message.text}
            </span>
          ) : (
            <ErrorText>{message.text}</ErrorText>
          ))}
      </div>
    </section>
  );
}
