import {
  Fragment,
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
  type ReactNode,
} from "react";
import { commands, type FileHit, type Segment } from "./bindings";
import { dismissLauncher, useLauncher } from "./useLauncher";
import { matchBuiltins, type BuiltinId } from "./launcher/builtins";
import { matchSpaces } from "./launcher/spaces";
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
  hit: FileHit,
  onError: (msg: string) => void,
  done: () => void,
) {
  const r = await commands.openFile(hit.path);
  if (r.status === "error") onError(r.error.message);
  else done();
}

/** La barra está en uno de tres modos. */
type Mode =
  | { kind: "list" }
  /** Un Espacio va a ejecutar comandos por primera vez: hay que confirmarlos. */
  | { kind: "confirm"; space: SpaceView; commands: PendingCommand[] }
  /** Alguna acción falló (o el Espacio no se pudo ejecutar): se explica qué. */
  | { kind: "result"; title: string; errors: string[] };

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
    const missing = fileHits
      .map((h) => h.path)
      .filter((p) => !(p in fileIcons));
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
  }, [fileHits, fileIcons]);

  // Los Espacios y el ajuste de iconos se recargan cada vez que se abre la barra, así
  // siempre reflejan lo último que se editó en Ajustes.
  function refresh() {
    return Promise.all([commands.listSpaces(), commands.getSettings()]).then(
      ([list, settings]) => {
        if (list.status === "ok") setSpaces(list.data);
        setIconsHidden(settings.iconsHidden);
      },
    );
  }

  useEffect(() => {
    let alive = true;
    void Promise.all([commands.listSpaces(), commands.getSettings()]).then(
      ([list, settings]) => {
        if (!alive) return; // el componente ya no existe: no tocar su estado
        if (list.status === "ok") setSpaces(list.data);
        setIconsHidden(settings.iconsHidden);
      },
    );
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

  useLauncher(
    rootRef,
    () => {
      reset();
      inputRef.current?.focus();
      void refresh();
    },
    // Al ocultarse se limpia ya, con la ventana invisible: al volver no se ve lo de antes.
    reset,
  );

  const launch = useCallback(async (view: SpaceView, approve: boolean) => {
    const result = await commands.runSpace(view.space.id, approve);
    if (result.status === "error") {
      setMode({
        kind: "result",
        title: view.space.name,
        errors: [result.error.message],
      });
      return;
    }
    const report = result.data;
    if (report.status === "needsApproval") {
      setMode({ kind: "confirm", space: view, commands: report.commands });
      return;
    }
    const errors = report.outcomes
      .filter((o) => o.error)
      .map((o) => `${o.label}: ${o.error}`);
    if (errors.length) {
      setMode({ kind: "result", title: view.space.name, errors });
    } else {
      dismissLauncher(rootRef.current);
    }
  }, []);

  const items = useMemo<Item[]>(() => {
    const fromSpaces = matchSpaces(
      query,
      spaces.map((v) => ({ ...v.space, view: v })),
    ).map(({ view }) => ({
      key: `space-${view.space.id}`,
      section: "spaces" as const,
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
    }));
    const files = fileHits.map((hit) => ({
      key: `file-${hit.fileId}`,
      section: "files" as const,
      title: <Highlighted segments={hit.nameSegments} />,
      // Con coincidencia en el contenido, el fragmento; si no, la carpeta.
      subtitle: hit.snippet.length ? (
        <Highlighted segments={hit.snippet} />
      ) : (
        shortFolder(hit.path)
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
          hit,
          (msg) => setMode({ kind: "result", title: hit.name, errors: [msg] }),
          () => dismissLauncher(rootRef.current),
        ),
    }));
    const builtins = matchBuiltins(query).map((c) => ({
      key: `builtin-${c.id}`,
      section: "commands" as const,
      title: c.title,
      subtitle: c.subtitle,
      icon: <span className="text-xl">{BUILTIN_ICON[c.id]}</span>,
      shortcut: c.shortcut,
      activate: () => openSettings(c.id === "newSpace"),
    }));
    // Espacios, luego archivos y, al final, los comandos de la propia app.
    return [...fromSpaces, ...files, ...builtins];
  }, [query, spaces, launch, fileHits, fileIcons]);

  const starts = useMemo(
    () => sectionStarts(items.map((i) => i.section)),
    [items],
  );

  const isEmpty = spaces.length === 0 && !query.trim();

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
        ) : mode.kind === "result" ? (
          <ResultPanel title={mode.title} errors={mode.errors} />
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

function ResultPanel({ title, errors }: { title: string; errors: string[] }) {
  return (
    <div role="alert" className="h-full overflow-y-auto p-5">
      <p className="mb-3 text-[15px]">«{title}»: algo no se pudo abrir</p>
      <ul className="space-y-1 text-sm text-danger select-text">
        {errors.map((e) => (
          <li key={e}>{e}</li>
        ))}
      </ul>
      <p className="mt-4 text-xs text-muted">
        Revisa el Espacio en Ajustes. Esc para cerrar.
      </p>
    </div>
  );
}

export default App;
