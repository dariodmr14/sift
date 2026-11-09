import { useEffect, useMemo, useRef, useState, type ReactNode } from "react";
import { commands } from "./bindings";
import { useLauncher } from "./useLauncher";
import {
  MOCK_RESULTS,
  filterResults,
  type ResultKind,
} from "./launcher/results";
import { matchBuiltins, type BuiltinId } from "./launcher/builtins";
import { moveSelection } from "./launcher/selection";

const KIND_ICON: Record<ResultKind, string> = {
  pdf: "📕",
  doc: "📘",
  note: "📝",
  folder: "📁",
  app: "🧩",
  web: "🌐",
  space: "✨",
};

const BUILTIN_ICON: Record<BuiltinId, string> = { settings: "⚙️" };

/** Lo que la lista sabe pintar y ejecutar, venga de donde venga el resultado. */
interface Item {
  key: string;
  title: string;
  subtitle: string;
  icon: ReactNode;
  shortcut?: string;
  activate: () => void;
}

function openSettings() {
  void commands.openSettings();
}

function App() {
  const [query, setQuery] = useState("");
  const [selected, setSelected] = useState(0);
  const inputRef = useRef<HTMLInputElement>(null);
  const listRef = useRef<HTMLUListElement>(null);

  const items = useMemo<Item[]>(() => {
    const results = filterResults(query, MOCK_RESULTS).map((r) => ({
      key: `mock-${r.id}`,
      title: r.title,
      subtitle: r.subtitle,
      icon: KIND_ICON[r.kind],
      activate: () => {
        // Fase 1: los resultados son de prueba. Abrirlos de verdad llega con los Espacios.
        console.info("abrir", r);
        void commands.hideLauncher();
      },
    }));
    const builtins = matchBuiltins(query).map((c) => ({
      key: `builtin-${c.id}`,
      title: c.title,
      subtitle: c.subtitle,
      icon: BUILTIN_ICON[c.id],
      shortcut: c.shortcut,
      activate: openSettings,
    }));
    // Los comandos van detrás: lo que buscas suele ser contenido, no la propia app.
    return [...results, ...builtins];
  }, [query]);

  useLauncher(() => {
    setQuery("");
    setSelected(0);
    inputRef.current?.focus();
  });

  // Que el elemento seleccionado siempre quede a la vista al moverse con las flechas.
  useEffect(() => {
    listRef.current
      ?.querySelector(`[data-index="${selected}"]`)
      ?.scrollIntoView({ block: "nearest" });
  }, [selected]);

  function onQueryChange(value: string) {
    setQuery(value);
    setSelected(0); // al escribir, la selección vuelve al primer resultado
  }

  function onKeyDown(e: React.KeyboardEvent<HTMLInputElement>) {
    // Ctrl+, abre los ajustes, como en VS Code o Chrome.
    if (e.ctrlKey && e.key === ",") {
      e.preventDefault();
      openSettings();
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
        items[selected]?.activate();
        break;
      case "Escape":
        e.preventDefault();
        void commands.hideLauncher();
        break;
    }
  }

  const activeId = items[selected]
    ? `result-${items[selected].key}`
    : undefined;

  return (
    <main className="flex h-full flex-col bg-(--surface)">
      <input
        ref={inputRef}
        autoFocus
        value={query}
        onChange={(e) => onQueryChange(e.currentTarget.value)}
        onKeyDown={onKeyDown}
        placeholder="Buscar en Sift…"
        spellCheck={false}
        role="combobox"
        aria-label="Buscar"
        aria-expanded={items.length > 0}
        aria-controls="results"
        aria-activedescendant={activeId}
        className="w-full border-b border-(--border) bg-transparent px-5 py-4 text-xl text-(--text) outline-none placeholder:text-(--text-muted)"
      />

      {items.length > 0 ? (
        <ul
          id="results"
          ref={listRef}
          role="listbox"
          aria-label="Resultados"
          className="flex-1 overflow-y-auto p-2"
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
              <span className="w-7 shrink-0 text-center text-xl" aria-hidden>
                {item.icon}
              </span>
              <span className="min-w-0 flex-1">
                <span className="block truncate text-[15px]">{item.title}</span>
                <span className="block truncate text-xs text-(--text-muted)">
                  {item.subtitle}
                </span>
              </span>
              {item.shortcut && (
                <kbd className="text-xs text-(--text-muted)" aria-hidden>
                  {item.shortcut}
                </kbd>
              )}
              {i === selected && !item.shortcut && (
                <kbd className="text-xs text-(--text-muted)" aria-hidden>
                  ↵
                </kbd>
              )}
            </li>
          ))}
        </ul>
      ) : (
        <p className="flex-1 p-6 text-center text-(--text-muted)">
          Sin resultados para «{query}»
        </p>
      )}

      <footer className="flex items-center gap-4 border-t border-(--border) px-5 py-2 text-xs text-(--text-muted)">
        <span>↑↓ moverse</span>
        <span>↵ abrir</span>
        <span>Esc cerrar</span>
        {/* Para quien use el ratón. Con teclado: Ctrl+, o escribir "ajustes". */}
        <button
          type="button"
          tabIndex={-1}
          onMouseDown={(e) => e.preventDefault()}
          onClick={openSettings}
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

export default App;
