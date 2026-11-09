import { useEffect, useMemo, useRef, useState, type ReactNode } from "react";
import { commands } from "./bindings";
import { useLauncher } from "./useLauncher";
import { matchBuiltins, type BuiltinId } from "./launcher/builtins";
import { matchSpaces } from "./launcher/spaces";
import { moveSelection } from "./launcher/selection";
import { SpaceIcon } from "./components/SpaceIcon";

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
  title: string;
  subtitle: string;
  icon: ReactNode;
  shortcut?: string;
  activate: () => void;
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

  useLauncher(() => {
    setQuery("");
    setSelected(0);
    setMode({ kind: "list" });
    inputRef.current?.focus();
    void refresh();
  });

  async function launch(view: SpaceView, approve: boolean) {
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
      void commands.hideLauncher();
    }
  }

  const items = useMemo<Item[]>(() => {
    const fromSpaces = matchSpaces(
      query,
      spaces.map((v) => ({ ...v.space, view: v })),
    ).map(({ view }) => ({
      key: `space-${view.space.id}`,
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
    const builtins = matchBuiltins(query).map((c) => ({
      key: `builtin-${c.id}`,
      title: c.title,
      subtitle: c.subtitle,
      icon: <span className="text-xl">{BUILTIN_ICON[c.id]}</span>,
      shortcut: c.shortcut,
      activate: () => openSettings(c.id === "newSpace"),
    }));
    // Los comandos van detrás: lo que buscas suele ser un Espacio, no la propia app.
    return [...fromSpaces, ...builtins];
  }, [query, spaces]);

  const isEmpty = spaces.length === 0 && !query.trim();

  // Que el elemento seleccionado siempre quede a la vista al moverse con las flechas.
  useEffect(() => {
    listRef.current
      ?.querySelector(`[data-index="${selected}"]`)
      ?.scrollIntoView({ block: "nearest" });
  }, [selected]);

  function onKeyDown(e: React.KeyboardEvent<HTMLInputElement>) {
    if (e.ctrlKey && (e.key === "," || e.key.toLowerCase() === "n")) {
      e.preventDefault(); // Ctrl+, ajustes; Ctrl+N crear Espacio
      openSettings(e.key !== ",");
      return;
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
        void commands.hideLauncher();
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
        void commands.hideLauncher();
        break;
    }
  }

  const activeId =
    mode.kind === "list" && items[selected]
      ? `result-${items[selected].key}`
      : undefined;

  return (
    <main className="flex h-full flex-col bg-(--surface)">
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
        placeholder="Buscar un Espacio…"
        spellCheck={false}
        role="combobox"
        aria-label="Buscar"
        aria-expanded={mode.kind === "list" && items.length > 0}
        aria-controls="results"
        aria-activedescendant={activeId}
        className="w-full border-b border-(--border) bg-transparent px-5 py-4 text-xl text-(--text) outline-none placeholder:text-(--text-muted)"
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
              <li
                key={item.key}
                id={`result-${item.key}`}
                data-index={i}
                role="option"
                aria-selected={i === selected}
                onMouseMove={() => setSelected(i)}
                onMouseDown={(e) => e.preventDefault()} // que el input no pierda el foco
                onClick={item.activate}
                className={`flex items-center gap-3 rounded-md px-3 py-2 ${
                  i === selected ? "bg-(--surface-selected)" : ""
                }`}
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
                  <span className="block truncate text-xs text-(--text-muted)">
                    {item.subtitle}
                  </span>
                </span>
                {item.shortcut ? (
                  <kbd className="text-xs text-(--text-muted)" aria-hidden>
                    {item.shortcut}
                  </kbd>
                ) : (
                  i === selected && (
                    <kbd className="text-xs text-(--text-muted)" aria-hidden>
                      ↵
                    </kbd>
                  )
                )}
              </li>
            ))}
          </ul>
        ) : (
          <p className="p-6 text-center text-(--text-muted)">
            Sin resultados para «{query}»
          </p>
        )}
      </div>

      <footer className="flex items-center gap-4 border-t border-(--border) px-5 py-2 text-xs text-(--text-muted)">
        {mode.kind === "confirm" ? (
          <>
            <span>↵ ejecutar</span>
            <span>Esc volver</span>
          </>
        ) : (
          <>
            <span>↑↓ moverse</span>
            <span>↵ abrir</span>
            <span>Ctrl+N nuevo Espacio</span>
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
      <p className="max-w-sm text-sm text-(--text-muted)">
        Un Espacio abre de golpe todo lo de un contexto: carpetas, webs, apps y
        comandos.
      </p>
      <button
        type="button"
        onMouseDown={(e) => e.preventDefault()}
        onClick={() => openSettings(true)}
        className="mt-2 rounded-md bg-(--surface-selected) px-4 py-2 text-sm"
      >
        Crear tu primer Espacio{" "}
        <kbd className="ml-2 text-(--text-muted)">↵</kbd>
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
      <p className="mb-4 text-xs text-(--text-muted)">
        Revisa que sea lo que esperas. Solo se pregunta una vez por comando: si
        cambia, se vuelve a preguntar.
      </p>
      <ul className="space-y-2">
        {commands.map((c) => (
          <li
            key={c.display}
            className="rounded-md bg-(--surface-selected) px-3 py-2 font-mono text-[13px] select-text"
          >
            {c.display}
            {c.command.cwd && (
              <span className="mt-1 block text-xs text-(--text-muted)">
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
      <ul className="space-y-1 text-sm text-red-400 select-text">
        {errors.map((e) => (
          <li key={e}>{e}</li>
        ))}
      </ul>
      <p className="mt-4 text-xs text-(--text-muted)">
        Revisa el Espacio en Ajustes. Esc para cerrar.
      </p>
    </div>
  );
}

export default App;
