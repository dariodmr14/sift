import { useCallback, useEffect, useState } from "react";
import {
  commands,
  events,
  type IndexStatus,
  type RootView,
  type UnreadGroupView,
} from "../bindings";
import { Button, ErrorText } from "./ui";
import { count, duration, files, since } from "../format";
import {
  folderName,
  shortParent,
  statusLine,
  statusNotes,
  unreadReason,
  type StatusLine,
} from "./folders";

/** Ajustes → Carpetas: dónde busca Sift y cómo va. Pensado para cualquiera. */
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

  const totalFiles = roots?.reduce((n, r) => n + r.files, 0) ?? 0;
  const line = statusLine(status, totalFiles, roots?.length ?? 0);

  return (
    <div className="h-full space-y-8 overflow-y-auto p-6">
      <section className="space-y-3">
        <header className="flex items-baseline justify-between gap-4">
          <h2 className="text-sm font-semibold tracking-wide text-muted uppercase">
            Carpetas
          </h2>
          <Button variant="primary" onClick={() => void add()}>
            + Añadir carpeta…
          </Button>
        </header>
        <p className="text-sm text-muted">
          Sift busca por nombre y por contenido en estas carpetas y en todo lo
          que tienen dentro. Nada sale de tu PC.
        </p>

        <StatusPanel
          line={line}
          running={status?.running ?? false}
          lastRun={status?.last?.seconds ?? undefined}
          error={status?.error}
          notes={statusNotes(status)}
        />

        {error && <ErrorText>{error}</ErrorText>}
        <ul className="divide-y divide-line rounded-window border border-line">
          {roots?.length === 0 && (
            <li className="p-4 text-sm text-muted">
              Aún no hay carpetas. Añade, por ejemplo, Documentos o la carpeta
              donde guardas tus trabajos.
            </li>
          )}
          {roots?.map((root) => (
            <RootRow
              key={root.id}
              root={root}
              confirming={confirmRemove === root.id}
              onAskRemove={() => setConfirmRemove(root.id)}
              onCancelRemove={() => setConfirmRemove(null)}
              onRemove={() => void remove(root.id)}
            />
          ))}
        </ul>
        <p className="text-xs text-muted">
          Sift solo mira lo que sirve para buscar: documentos, textos, código y,
          por su nombre, imágenes y vídeos. Se salta programas y archivos
          internos de otras herramientas. Por ahora lee el contenido de textos y
          notas, código, PDF, Word y PowerPoint; las hojas de cálculo se
          encuentran por su nombre. Lee en segundo plano y se pausa si estás
          usando mucho el PC. Quitar una carpeta solo hace que Sift deje de
          buscar en ella: tus archivos no se tocan.
        </p>
      </section>

      <Advanced />
    </div>
  );
}

function StatusPanel({
  line,
  running,
  lastRun,
  error,
  notes,
}: {
  line: StatusLine;
  running: boolean;
  lastRun: number | undefined;
  error: string | null | undefined;
  notes: string[];
}) {
  return (
    <div className="space-y-2 rounded-window border border-line px-4 py-3">
      <div className="flex items-center justify-between gap-4">
        {/* Se anuncia solo el texto, no cada avance de la barra. */}
        <p role="status" className="text-sm font-medium">
          {line.text}
        </p>
        <Button
          variant="ghost"
          disabled={running}
          onClick={() => void commands.reindex()}
          title="Sift ya revisa solo al arrancar y al cambiar las carpetas"
        >
          Revisar ahora
        </Button>
      </div>
      {line.progress !== null && <ProgressBar value={line.progress} />}
      {!running && lastRun !== undefined && (
        <p className="text-xs text-muted">
          La última revisión completa tardó {duration(lastRun)}. Desde entonces,
          Sift ve tus cambios al momento y solo vuelve a leer lo que cambia.
        </p>
      )}
      {notes.map((note) => (
        <p key={note} className="text-xs text-muted">
          {note}
        </p>
      ))}
      {error && <ErrorText>{error}</ErrorText>}
    </div>
  );
}

/**
 * Una carpeta: nombre, dónde está, cuántos archivos y cuándo se actualizó. Si algunos no
 * se pudieron leer, su número es un botón con flecha que despliega, debajo de la fila,
 * los motivos de **esa** carpeta con unos pocos ejemplos (no todos: pueden ser miles).
 */
function RootRow({
  root,
  confirming,
  onAskRemove,
  onCancelRemove,
  onRemove,
}: {
  root: RootView;
  confirming: boolean;
  onAskRemove: () => void;
  onCancelRemove: () => void;
  onRemove: () => void;
}) {
  const [open, setOpen] = useState(false);
  const [groups, setGroups] = useState<UnreadGroupView[] | null>(null);
  const panelId = `unread-${root.id}`;

  // Se piden al abrir, y de nuevo si cambia el número (una revisión acaba de terminar).
  useEffect(() => {
    if (!open) return;
    void commands.unreadFiles(root.id).then((r) => {
      if (r.status === "ok") setGroups(r.data);
    });
  }, [open, root.id, root.errors]);

  return (
    <li className="px-4 py-3">
      <div className="flex items-center gap-3">
        <span className="min-w-0 flex-1">
          <span className="flex items-baseline gap-2">
            <span className="truncate text-sm font-medium">
              {folderName(root.path)}
            </span>
            <span className="truncate text-xs text-muted" title={root.path}>
              {shortParent(root.path)}
            </span>
          </span>
          <span className="block text-xs text-muted">
            {root.missing ? (
              <span className="text-danger">
                No se encuentra la carpeta. ¿Está en un disco desconectado?
              </span>
            ) : (
              <>
                {files(root.files)}
                {root.lastIndexedAt
                  ? ` · actualizada ${since(root.lastIndexedAt)}`
                  : " · aún sin revisar"}
              </>
            )}
          </span>
        </span>
        {confirming ? (
          <span className="flex items-center gap-2 text-sm">
            ¿Dejar de buscar aquí?
            <Button variant="danger" onClick={onRemove}>
              Sí, quitar
            </Button>
            <Button variant="ghost" onClick={onCancelRemove}>
              No
            </Button>
          </span>
        ) : (
          <Button
            variant="ghost"
            onClick={onAskRemove}
            aria-label={`Quitar ${root.path}`}
          >
            Quitar
          </Button>
        )}
      </div>
      {!root.missing && root.errors > 0 && (
        <>
          <button
            type="button"
            aria-expanded={open}
            aria-controls={panelId}
            onClick={() => setOpen(!open)}
            className="mt-1 flex items-center gap-1.5 rounded-item text-xs text-muted outline-none select-none hover:text-fg focus-visible:ring-2 focus-visible:ring-accent"
          >
            <span
              aria-hidden
              className={`inline-block transition-transform duration-150 motion-reduce:transition-none ${
                open ? "rotate-90" : ""
              }`}
            >
              ›
            </span>
            {root.errors === 1
              ? "1 archivo no se pudo leer"
              : `${count(root.errors)} archivos no se pudieron leer`}
          </button>
          {open && (
            <ul id={panelId} className="space-y-3 pt-2 pl-4 text-sm">
              {groups?.map((g) => {
                const reason = unreadReason(g.kind);
                return (
                  <li key={g.kind}>
                    <p>
                      <span className="font-medium">{reason.label}</span>
                      <span className="text-muted"> · {count(g.count)}</span>
                    </p>
                    <p className="text-xs text-muted">{reason.hint}</p>
                    <ul className="mt-1 space-y-0.5">
                      {g.examples.map((path) => (
                        <li
                          key={path}
                          title={path}
                          className="truncate font-mono text-xs text-muted"
                        >
                          {folderName(path)}
                        </li>
                      ))}
                      {g.count > g.examples.length && (
                        <li className="text-xs text-muted">
                          …y {count(g.count - g.examples.length)} más
                        </li>
                      )}
                    </ul>
                  </li>
                );
              })}
            </ul>
          )}
        </>
      )}
    </li>
  );
}

/** Barra de progreso: con total, se llena (solo `transform`); sin él, va y viene. */
function ProgressBar({ value }: { value: number | "indeterminate" }) {
  const indeterminate = value === "indeterminate";
  return (
    <div
      role="progressbar"
      aria-valuemin={0}
      aria-valuemax={100}
      aria-valuenow={indeterminate ? undefined : Math.round(value * 100)}
      className="h-1 overflow-hidden rounded-item bg-hover"
    >
      {indeterminate ? (
        <div className="sift-progress h-full w-1/3 rounded-item bg-accent" />
      ) : (
        <div
          className="h-full origin-left rounded-item bg-accent transition-transform duration-200 motion-reduce:transition-none"
          style={{ transform: `scaleX(${value})` }}
        />
      )}
    </div>
  );
}

/** «Buscar también imágenes y vídeos por su nombre» (activado por defecto). */
function MediaToggle() {
  const [enabled, setEnabled] = useState<boolean | null>(null);
  useEffect(() => {
    void commands.getIndexMedia().then(setEnabled);
  }, []);
  return (
    <label className="flex items-start gap-3 text-sm">
      <input
        type="checkbox"
        checked={enabled ?? true}
        disabled={enabled === null}
        onChange={(e) => {
          void commands.setIndexMedia(e.currentTarget.checked).then((r) => {
            if (r.status === "ok") setEnabled(r.data);
          });
        }}
        className="mt-1 size-4 accent-accent"
      />
      <span>
        Buscar también imágenes y vídeos por su nombre
        <span className="block text-xs text-muted">
          Su contenido no se lee. No aparecen los que están en carpetas que Sift
          se salta (las que otros programas crean para su uso interno).
        </span>
      </span>
    </label>
  );
}

/** Patrones a ignorar: vienen bien de serie, así que se esconden en «Avanzado». */
function Advanced() {
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
    setMessage({ ok: true, text: "Guardado. Sift está revisando de nuevo." });
  }

  return (
    <details className="group space-y-3">
      <summary className="flex cursor-pointer list-none items-center gap-2 rounded-item text-sm font-semibold tracking-wide text-muted uppercase outline-none select-none hover:text-fg focus-visible:ring-2 focus-visible:ring-accent">
        <span
          aria-hidden
          className="inline-block transition-transform duration-150 group-open:rotate-90 motion-reduce:transition-none"
        >
          ›
        </span>
        Avanzado
      </summary>
      <div className="space-y-3 pt-3">
        <MediaToggle />
        <p className="text-sm text-muted">
          Además, Sift se salta por su cuenta las carpetas que crean las
          herramientas y no tú: dependencias (
          <code className="font-mono">node_modules</code>), compilados (
          <code className="font-mono">bin</code>,{" "}
          <code className="font-mono">build</code>), entornos de Python y
          cachés. Normalmente no tienes que tocar nada. Si quieres que no mire
          alguna carpeta o tipo de archivo más, añádelo aquí.
        </p>
        <label className="block space-y-1">
          <span className="text-xs text-muted">
            Uno por línea. Un nombre (
            <code className="font-mono">Borradores</code>,{" "}
            <code className="font-mono">*.tmp</code>) se salta esté donde esté;
            con barra (<code className="font-mono">Antiguo/*</code>) se cuenta
            desde la carpeta añadida.
          </span>
          <textarea
            aria-label="Qué no mirar"
            spellCheck={false}
            rows={8}
            value={text}
            onChange={(e) => setText(e.currentTarget.value)}
            className="w-full rounded-item border border-line bg-field px-3 py-2 font-mono text-sm outline-none focus:border-accent"
          />
        </label>
        <div className="flex items-center gap-3">
          <Button
            variant="primary"
            disabled={text === saved}
            onClick={() => void save()}
          >
            Guardar
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
      </div>
    </details>
  );
}
