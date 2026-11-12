import {
  Fragment,
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
  type ReactNode,
} from "react";
import {
  commands,
  events,
  type FileHit,
  type RecentFile,
  type Segment,
  type SpaceProgress_Deserialize as SpaceProgress,
} from "./bindings";
import { dismissLauncher, useLauncher } from "./useLauncher";
import { matchBuiltins, type BuiltinId } from "./launcher/builtins";
import { mixResults } from "./launcher/mix";
import {
  progressText,
  summarize,
  type SummaryLine,
} from "./launcher/runSummary";
import { moveSelection } from "./launcher/selection";
import { SpaceIcon } from "./components/SpaceIcon";
import {
  SECTION_LABEL,
  isLatest,
  sectionStarts,
  shortFolder,
  type Section,
} from "./launcher/files";

type Ok<T> = Extract<T, { status: "ok" }> extends { data: infer D } ? D : never;
type SpaceView = Ok<Awaited<ReturnType<typeof commands.listSpaces>>>[number];
type RunReport = Ok<Awaited<ReturnType<typeof commands.runSpace>>>;
type PendingCommand = Extract<
  RunReport,
  { status: "needsApproval" }
>["commands"][number];

const BUILTIN_ICON: Record<BuiltinId, string> = {
  settings: "⚙️",
  newSpace: "➕",
};

/** Lo que la lista sabe pintar y ejecutar, venga de donde venga. */
interface Item {
  key: string;
  section: Section;
  title: ReactNode;
  subtitle: ReactNode;
  icon: ReactNode;
  shortcut?: string;
  /** Ruta del archivo (solo resultados de archivos): Ctrl+↵ carpeta, Ctrl+Shift+C copiar. */
  path?: string;
  activate: () => void;
}

/** Texto con las partes encontradas resaltadas (los trozos vienen de Rust). */
function Highlighted({ segments }: { segments: Segment[] }) {
  return (
    <>
      {segments.map((s, i) =>
        s.hit ? (
          <mark key={i} className="rounded-[2px] bg-accent/25 text-fg">
            {s.text}
          </mark>
        ) : (
          <span key={i}>{s.text}</span>
        ),
      )}
    </>
  );
}

/** Abre un archivo encontrado; si falla, el motivo va al panel de resultado. */
async function openFile(
  path: string,
  onError: (msg: string) => void,
  done: () => void,
) {
  const r = await commands.openFile(path);
  if (r.status === "error") onError(r.error.message);
  else done();
}

/** La barra está en uno de tres modos. */
type Mode =
  | { kind: "list" }
  /** Un Espacio va a ejecutar comandos por primera vez: hay que confirmarlos. */
  | { kind: "confirm"; space: SpaceView; commands: PendingCommand[] }
  /** Un Espacio en marcha (con esperas puede tardar): por qué paso va. Esc cancela. */
  | {
      kind: "running";
      title: string;
      progress: SpaceProgress | null;
      cancelling: boolean;
    }
  /** Algo falló, se paró o se canceló: qué se ejecutó y qué no. */
  | { kind: "result"; title: string; headline: string; lines: SummaryLine[] };

const openSettings = (newSpace = false) => void commands.openSettings(newSpace);

function subtitleFor(view: SpaceView): string {
  const n = view.space.actions.length;
  const actions = n === 1 ? "1 acción" : `${n} acciones`;
  return view.space.aliases.length
    ? `${actions} · ${view.space.aliases.join(", ")}`
    : actions;
}

function App() {
  const [query, setQuery] = useState("");
  const [selected, setSelected] = useState(0);
  const [spaces, setSpaces] = useState<SpaceView[]>([]);
  const [iconsHidden, setIconsHidden] = useState(false);
  const [mode, setMode] = useState<Mode>({ kind: "list" });
  const inputRef = useRef<HTMLInputElement>(null);
  const listRef = useRef<HTMLUListElement>(null);
  const rootRef = useRef<HTMLElement>(null);
  const [fileHits, setFileHits] = useState<FileHit[]>([]);
  // «Abierto recientemente», para la barra vacía. No se borra al ocultarse: al abrir se
  // pinta con la lista de la vez anterior y se pone al día después, sin esperar a Rust.
  const [recent, setRecent] = useState<RecentFile[]>([]);
  const [fileIcons, setFileIcons] = useState<Record<string, string | null>>({});
  // Número de la última búsqueda pedida: las respuestas de búsquedas anteriores se ignoran.
  const searchSeq = useRef(0);

  // Búsqueda de archivos en Rust a cada tecla (es rápida: < 50 ms).
  useEffect(() => {
    const seq = ++searchSeq.current;
    if (!query.trim()) {
      // Sin consulta no hay archivos que mostrar (se limpia en la siguiente vuelta).
      void Promise.resolve().then(
        () => isLatest(seq, searchSeq.current) && setFileHits([]),
      );
      return;
    }
    void commands.searchFiles(query, seq).then((r) => {
      if (r.status !== "ok" || !isLatest(r.data.seq, searchSeq.current)) return;
      setFileHits(r.data.results.hits);
    });
  }, [query]);

  // Iconos de Windows de los archivos encontrados (una vez por ruta).
  useEffect(() => {
    const missing = [...fileHits, ...recent]
      .map((h) => h.path)
      .filter((p, i, all) => !(p in fileIcons) && all.indexOf(p) === i);
    if (missing.length === 0) return;
    let alive = true;
    void commands.getIcons(missing).then((r) => {
      if (!alive || r.status !== "ok") return;
      setFileIcons((prev) => ({
        ...prev,
        ...Object.fromEntries(missing.map((p, i) => [p, r.data[i] ?? null])),
      }));
    });
    return () => {
      alive = false;
    };
  }, [fileHits, recent, fileIcons]);

  // Los Espacios y el ajuste de iconos se recargan cada vez que se abre la barra, así
  // siempre reflejan lo último que se editó en Ajustes.
  function refresh() {
    return Promise.all([
      commands.listSpaces(),
      commands.getSettings(),
      commands.recentFiles(),
    ]).then(([list, settings, opened]) => {
      if (list.status === "ok") setSpaces(list.data);
      setIconsHidden(settings.iconsHidden);
      if (opened.status === "ok") setRecent(opened.data);
    });
  }

  useEffect(() => {
    let alive = true;
    void Promise.all([
      commands.listSpaces(),
      commands.getSettings(),
      commands.recentFiles(),
    ]).then(([list, settings, opened]) => {
      if (!alive) return; // el componente ya no existe: no tocar su estado
      if (list.status === "ok") setSpaces(list.data);
      setIconsHidden(settings.iconsHidden);
      if (opened.status === "ok") setRecent(opened.data);
    });
    return () => {
      alive = false;
    };
  }, []);

  const dismiss = () => dismissLauncher(rootRef.current);
  const reset = () => {
    setQuery("");
    setSelected(0);
    setMode({ kind: "list" });
  };

  // Un Espacio con esperas puede seguir en marcha con la barra oculta. Mientras tanto no
  // se limpia nada, y si al acabar hay algo que contar, se enseña al volver a abrirla.
  const running = useRef(false);
  const visible = useRef(true);
  const keepResult = useRef(false);
  const cancelRequested = useRef(false);

  useLauncher(
    rootRef,
    () => {
      visible.current = true;
      if (running.current || keepResult.current) {
        keepResult.current = false;
      } else {
        reset();
      }
      inputRef.current?.focus();
      void refresh();
    },
    // Al ocultarse se limpia ya, con la ventana invisible: al volver no se ve lo de antes.
    () => {
      visible.current = false;
      if (!running.current) reset();
    },
  );

  // Por qué paso va el Espacio en marcha.
  useEffect(() => {
    const stop = events.spaceProgress.listen(({ payload }) =>
      setMode((m) => (m.kind === "running" ? { ...m, progress: payload } : m)),
    );
    return () => void stop.then((f) => f());
  }, []);

  const launch = useCallback(async (view: SpaceView, approve: boolean) => {
    const title = view.space.name;
    running.current = true;
    cancelRequested.current = false;
    // El panel «en marcha» solo si tarda: un Espacio normal se abre al instante y no
    // debe parpadear.
    const slow = setTimeout(
      () =>
        setMode({ kind: "running", title, progress: null, cancelling: false }),
      300,
    );
    const result = await commands.runSpace(view.space.id, approve);
    clearTimeout(slow);
    running.current = false;
    if (result.status === "error") {
      setMode({
        kind: "result",
        title,
        headline: "No se pudo abrir",
        lines: [
          { label: title, state: "failed", detail: result.error.message },
        ],
      });
      return;
    }
    const report = result.data;
    if (report.status === "needsApproval") {
      setMode({ kind: "confirm", space: view, commands: report.commands });
      return;
    }
    const summary = summarize(report.outcomes, cancelRequested.current);
    if (summary.needsAttention) {
      setMode({ kind: "result", title, ...summary });
      // Con la barra oculta, que lo vea al volver a abrirla.
      if (!visible.current) keepResult.current = true;
    } else if (visible.current) {
      dismissLauncher(rootRef.current);
    } else {
      reset();
    }
  }, []);

  const items = useMemo<Item[]>(() => {
    const mixed = mixResults(
      query,
      spaces.map((v) => ({
        name: v.space.name,
        aliases: v.space.aliases,
        boost: v.boost ?? 1,
        view: v,
      })),
      fileHits,
    );
    const spaceItem = (view: SpaceView): Item => ({
      key: `space-${view.space.id}`,
      section: "spaces",
      title: view.space.name,
      subtitle: subtitleFor(view),
      icon: (
        <SpaceIcon
          icon={view.space.icon}
          autoIcon={view.autoIcon}
          name={view.space.name}
        />
      ),
      activate: () => void launch(view, false),
    });
    const fileItem = (hit: FileHit): Item => ({
      key: `file-${hit.fileId}`,
      section: "files",
      title: <Highlighted segments={hit.nameSegments} />,
      // Con coincidencia en el contenido, el fragmento; si no, la carpeta. Lo que llega
      // solo por significado no tiene palabras que resaltar: se dice, para que se
      // entienda por qué está ahí.
      subtitle: (
        <>
          {hit.byMeaning && (
            <span
              className="text-accent"
              title="No contiene tus palabras: Sift lo ha encontrado por su significado"
            >
              Por significado ·{" "}
            </span>
          )}
          {hit.snippet.length ? (
            <Highlighted segments={hit.snippet} />
          ) : (
            shortFolder(hit.path)
          )}
        </>
      ),
      icon: fileIcons[hit.path] ? (
        <img
          src={fileIcons[hit.path]!}
          alt=""
          className="size-6"
          draggable={false}
        />
      ) : (
        <span className="size-6" />
      ),
      path: hit.path,
      activate: () =>
        void openFile(
          hit.path,
          (msg) =>
            setMode({
              kind: "result",
              title: hit.name,
              headline: "No se pudo abrir",
              lines: [{ label: hit.name, state: "failed", detail: msg }],
            }),
          () => dismissLauncher(rootRef.current),
        ),
    });
    const toItem = (m: (typeof mixed)[number]) =>
      m.kind === "space" ? spaceItem(m.item.view) : fileItem(m.item);
    // Barra vacía: los Espacios y lo último que abriste, para llegar sin escribir.
    if (!query.trim()) {
      const opened = recent.map((r): Item => ({
        key: `recent-${r.fileId}`,
        section: "recent",
        title: r.name,
        subtitle: shortFolder(r.path),
        icon: fileIcons[r.path] ? (
          <img
            src={fileIcons[r.path]!}
            alt=""
            className="size-6"
            draggable={false}
          />
        ) : (
          <span className="size-6" />
        ),
        path: r.path,
        activate: () =>
          void openFile(
            r.path,
            (msg) =>
              setMode({
                kind: "result",
                title: r.name,
                headline: "No se pudo abrir",
                lines: [{ label: r.name, state: "failed", detail: msg }],
              }),
            () => dismissLauncher(rootRef.current),
          ),
      }));
      return [...mixed.map(toItem), ...opened];
    }
    const builtins = matchBuiltins(query).map((c) => ({
      key: `builtin-${c.id}`,
      section: "commands" as const,
      title: c.title,
      subtitle: c.subtitle,
      icon: <span className="text-xl">{BUILTIN_ICON[c.id]}</span>,
      shortcut: c.shortcut,
      activate: () => openSettings(c.id === "newSpace"),
    }));
    // Primero los Espacios que encajan exactamente, luego los comandos de la propia app y,
    // al final, archivos con los Espacios que encajan a medias intercalados (ver
    // `mixResults`). Los comandos solo salen si lo escrito encaja con su nombre
    // («ajustes»), y son uno o dos: detrás de cientos de archivos no se encontrarían.
    const exact = mixed.filter((m) => m.kind === "space" && m.exact);
    const rest = mixed.filter((m) => !(m.kind === "space" && m.exact));
    const list = [...exact.map(toItem), ...builtins, ...rest.map(toItem)];
    // Sin cabeceras (Espacios intercalados entre archivos), cada Espacio dice qué es.
    if (sectionStarts(list.map((i) => i.section)).size === 0) {
      return list.map((i) =>
        i.section === "spaces"
          ? { ...i, subtitle: <>Espacio · {i.subtitle}</> }
          : i,
      );
    }
    return list;
  }, [query, spaces, launch, fileHits, recent, fileIcons]);

  const starts = useMemo(
    () => sectionStarts(items.map((i) => i.section)),
    [items],
  );

  const isEmpty = spaces.length === 0 && recent.length === 0 && !query.trim();

  // Que el elemento seleccionado siempre quede a la vista al moverse con las flechas.
  useEffect(() => {
    listRef.current
      ?.querySelector(`[data-index="${selected}"]`)
      ?.scrollIntoView({ block: "nearest" });
  }, [selected]);

  function onKeyDown(e: React.KeyboardEvent<HTMLInputElement>) {
    // Por la tecla física (`code`) y no por el carácter (`key`): con el teclado español
    // la coma no siempre llega como "," con Ctrl pulsado.
    if (e.ctrlKey && (e.code === "Comma" || e.code === "KeyN")) {
      e.preventDefault(); // Ctrl+, ajustes; Ctrl+N crear Espacio
      openSettings(e.code === "KeyN");
      return;
    }
    const current = items[selected];
    if (mode.kind === "list" && current?.path) {
      if (e.ctrlKey && e.key === "Enter") {
        e.preventDefault(); // Ctrl+↵: abrir la carpeta con el archivo seleccionado
        void commands.revealFile(current.path).then(() => dismiss());
        return;
      }
      if (e.ctrlKey && e.shiftKey && e.code === "KeyC") {
        e.preventDefault(); // Ctrl+Shift+C: copiar la ruta
        void navigator.clipboard.writeText(current.path).then(() => dismiss());
        return;
      }
    }
    if (mode.kind === "confirm") {
      if (e.key === "Enter") {
        e.preventDefault();
        void launch(mode.space, true);
      } else if (e.key === "Escape") {
        e.preventDefault();
        setMode({ kind: "list" });
      }
      return;
    }
    if (mode.kind === "running") {
      // Esc cancela: lo ya abierto sigue abierto; al acabar, el resumen.
      if (e.key === "Escape") {
        e.preventDefault();
        cancelRequested.current = true;
        void commands.cancelSpaceRun();
        setMode({ ...mode, cancelling: true });
      }
      return;
    }
    if (mode.kind === "result") {
      if (e.key === "Enter" || e.key === "Escape") {
        e.preventDefault();
        dismiss();
      }
      return;
    }
    switch (e.key) {
      case "ArrowDown":
      case "ArrowUp":
        e.preventDefault(); // que el cursor del input no salte al principio o al final
        setSelected((i) =>
          moveSelection(i, e.key === "ArrowDown" ? 1 : -1, items.length),
        );
        break;
      case "Enter":
        e.preventDefault();
        if (isEmpty) openSettings(true);
        else items[selected]?.activate();
        break;
      case "Escape":
        e.preventDefault();
        dismiss();
        break;
    }
  }

  const activeId =
    mode.kind === "list" && items[selected]
      ? `result-${items[selected].key}`
      : undefined;

  return (
    <main ref={rootRef} className="sift-from flex h-full flex-col bg-surface">
      <input
        ref={inputRef}
        autoFocus
        value={query}
        onChange={(e) => {
          setQuery(e.currentTarget.value);
          setSelected(0); // al escribir, la selección vuelve al primer resultado
          setMode({ kind: "list" });
        }}
        onKeyDown={onKeyDown}
        // Mientras se abre un Espacio, escribir no cambia nada (Esc sigue cancelando).
        readOnly={mode.kind === "running"}
        placeholder="Buscar en Sift…"
        spellCheck={false}
        role="combobox"
        aria-label="Buscar"
        aria-expanded={mode.kind === "list" && items.length > 0}
        aria-controls="results"
        aria-activedescendant={activeId}
        className="w-full border-b border-line bg-transparent px-5 py-4 text-xl text-fg outline-none placeholder:text-muted"
      />

      <div className="min-h-0 flex-1">
        {mode.kind === "confirm" ? (
          <ConfirmPanel
            space={mode.space.space.name}
            commands={mode.commands}
          />
        ) : mode.kind === "running" ? (
          <RunningPanel
            title={mode.title}
            progress={mode.progress}
            cancelling={mode.cancelling}
          />
        ) : mode.kind === "result" ? (
          <ResultPanel
            title={mode.title}
            headline={mode.headline}
            lines={mode.lines}
          />
        ) : isEmpty ? (
          <EmptyState />
        ) : items.length > 0 ? (
          <ul
            id="results"
            ref={listRef}
            role="listbox"
            aria-label="Resultados"
            className="h-full overflow-y-auto p-2"
          >
            {items.map((item, i) => (
              <Fragment key={item.key}>
                {starts.has(i) && (
                  // Cabecera de sección: no se puede seleccionar (no es una opción).
                  <li
                    role="presentation"
                    className="px-3 pt-2 pb-1 text-[11px] font-semibold tracking-wide text-muted uppercase first:pt-0"
                  >
                    {SECTION_LABEL[item.section]}
                  </li>
                )}
                <li
                  id={`result-${item.key}`}
                  data-index={i}
                  role="option"
                  aria-selected={i === selected}
                  onMouseMove={() => setSelected(i)}
                  onMouseDown={(e) => e.preventDefault()} // que el input no pierda el foco
                  onClick={item.activate}
                  className="sift-option flex items-center gap-3 px-3 py-2"
                >
                  {!iconsHidden && (
                    <span
                      className="flex w-7 shrink-0 items-center justify-center"
                      aria-hidden
                    >
                      {item.icon}
                    </span>
                  )}
                  <span className="min-w-0 flex-1">
                    <span className="block truncate text-[15px]">
                      {item.title}
                    </span>
                    <span className="block truncate text-xs text-muted">
                      {item.subtitle}
                    </span>
                  </span>
                  {item.shortcut ? (
                    <kbd className="text-xs text-muted" aria-hidden>
                      {item.shortcut}
                    </kbd>
                  ) : (
                    i === selected && (
                      <kbd className="text-xs text-muted" aria-hidden>
                        ↵
                      </kbd>
                    )
                  )}
                </li>
              </Fragment>
            ))}
          </ul>
        ) : (
          <p className="p-6 text-center text-muted">
            Sin resultados para «{query}»
          </p>
        )}
      </div>

      <footer className="flex items-center gap-4 border-t border-line px-5 py-2 text-xs text-muted">
        {mode.kind === "confirm" ? (
          <>
            <span>↵ ejecutar</span>
            <span>Esc volver</span>
          </>
        ) : mode.kind === "running" ? (
          <span>Esc cancelar (lo ya abierto sigue abierto)</span>
        ) : mode.kind === "result" ? (
          <span>Esc cerrar</span>
        ) : (
          <>
            <span>↑↓ moverse</span>
            <span>↵ abrir</span>
            {items[selected]?.path ? (
              <>
                <span>Ctrl+↵ carpeta</span>
                <span>Ctrl+Shift+C copiar ruta</span>
              </>
            ) : (
              <span>Ctrl+N nuevo Espacio</span>
            )}
          </>
        )}
        {/* Para quien use el ratón. Con teclado: Ctrl+, o escribir "ajustes". */}
        <button
          type="button"
          tabIndex={-1}
          onMouseDown={(e) => e.preventDefault()}
          onClick={() => openSettings()}
          title="Ajustes (Ctrl+,)"
          aria-label="Ajustes"
          className="ml-auto rounded px-1 text-base leading-none opacity-60 hover:opacity-100"
        >
          ⚙
        </button>
      </footer>
    </main>
  );
}

function EmptyState() {
  return (
    <div className="flex h-full flex-col items-center justify-center gap-3 p-6 text-center">
      <p className="text-lg">Aún no tienes Espacios</p>
      <p className="max-w-sm text-sm text-muted">
        Un Espacio abre de golpe todo lo de un contexto: carpetas, webs, apps y
        comandos.
      </p>
      <button
        type="button"
        onMouseDown={(e) => e.preventDefault()}
        onClick={() => openSettings(true)}
        className="mt-2 rounded-item bg-selected px-4 py-2 text-sm"
      >
        Crear tu primer Espacio <kbd className="ml-2 text-muted">↵</kbd>
      </button>
    </div>
  );
}

function ConfirmPanel({
  space,
  commands,
}: {
  space: string;
  commands: PendingCommand[];
}) {
  return (
    <div
      role="alertdialog"
      aria-labelledby="confirm-title"
      className="h-full overflow-y-auto p-5"
    >
      <p id="confirm-title" className="mb-1 text-[15px]">
        «{space}» va a ejecutar{" "}
        {commands.length === 1 ? "un comando" : "estos comandos"} por primera
        vez
      </p>
      <p className="mb-4 text-xs text-muted">
        Revisa que sea lo que esperas. Solo se pregunta una vez por comando: si
        cambia, se vuelve a preguntar.
      </p>
      <ul className="space-y-2">
        {commands.map((c) => (
          <li
            key={c.display}
            className="rounded-item bg-selected px-3 py-2 font-mono text-[13px] select-text"
          >
            {c.display}
            {c.command.cwd && (
              <span className="mt-1 block text-xs text-muted">
                en {c.command.cwd}
              </span>
            )}
          </li>
        ))}
      </ul>
    </div>
  );
}

function RunningPanel({
  title,
  progress,
  cancelling,
}: {
  title: string;
  progress: SpaceProgress | null;
  cancelling: boolean;
}) {
  return (
    <div role="status" aria-live="polite" className="h-full p-5">
      <p className="mb-1 text-[15px]">Abriendo «{title}»</p>
      <p className="text-sm text-muted">
        {cancelling
          ? "Cancelando…"
          : progress
            ? progressText({ ...progress, waited: progress.waited ?? null })
            : "Empezando…"}
      </p>
    </div>
  );
}

const LINE_MARK: Record<SummaryLine["state"], { mark: string; cls: string }> = {
  ok: { mark: "✓", cls: "text-success" },
  failed: { mark: "✕", cls: "text-danger" },
  cancelled: { mark: "✕", cls: "text-muted" },
  notRun: { mark: "–", cls: "text-muted" },
};

function ResultPanel({
  title,
  headline,
  lines,
}: {
  title: string;
  headline: string;
  lines: SummaryLine[];
}) {
  return (
    <div role="alert" className="h-full overflow-y-auto p-5">
      <p className="mb-3 text-[15px]">
        «{title}»: {headline}
      </p>
      <ul className="space-y-1.5 text-sm select-text">
        {lines.map((l, i) => (
          <li key={i} className="flex gap-2">
            <span
              aria-hidden
              className={`w-4 shrink-0 text-center ${LINE_MARK[l.state].cls}`}
            >
              {LINE_MARK[l.state].mark}
            </span>
            <span className="min-w-0">
              <span className={l.state === "notRun" ? "text-muted" : ""}>
                {l.label}
                {l.state === "notRun" && " (no se ejecutó)"}
              </span>
              {l.detail && (
                <span
                  className={`block font-mono text-xs whitespace-pre-wrap ${
                    l.state === "failed" ? "text-danger" : "text-muted"
                  }`}
                >
                  {l.detail}
                </span>
              )}
            </span>
          </li>
        ))}
      </ul>
      <p className="mt-4 text-xs text-muted">
        Revisa el Espacio en Ajustes si hace falta.
      </p>
    </div>
  );
}

export default App;
