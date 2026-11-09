import { useEffect, useMemo, useRef, useState } from "react";
import { commands } from "./bindings";
import { useLauncher } from "./useLauncher";
import {
  MOCK_RESULTS,
  filterResults,
  type ResultKind,
  type SearchResult,
} from "./launcher/results";
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

function App() {
  const [query, setQuery] = useState("");
  const [selected, setSelected] = useState(0);
  const inputRef = useRef<HTMLInputElement>(null);
  const listRef = useRef<HTMLUListElement>(null);

  const results = useMemo(() => filterResults(query, MOCK_RESULTS), [query]);

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

  function activate(result: SearchResult | undefined) {
    if (!result) return;
    // Fase 1: los resultados son de prueba. Abrirlos de verdad llega en la Fase 2.
    console.info("abrir", result);
    void commands.hideLauncher();
  }

  function onQueryChange(value: string) {
    setQuery(value);
    setSelected(0); // al escribir, la selección vuelve al primer resultado
  }

  function onKeyDown(e: React.KeyboardEvent<HTMLInputElement>) {
    switch (e.key) {
      case "ArrowDown":
      case "ArrowUp":
        e.preventDefault(); // que el cursor del input no salte al principio o al final
        setSelected((i) =>
          moveSelection(i, e.key === "ArrowDown" ? 1 : -1, results.length),
        );
        break;
      case "Enter":
        e.preventDefault();
        activate(results[selected]);
        break;
      case "Escape":
        e.preventDefault();
        void commands.hideLauncher();
        break;
    }
  }

  const activeId = results[selected]
    ? `result-${results[selected].id}`
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
        aria-expanded={results.length > 0}
        aria-controls="results"
        aria-activedescendant={activeId}
        className="w-full border-b border-(--border) bg-transparent px-5 py-4 text-xl text-(--text) outline-none placeholder:text-(--text-muted)"
      />

      {results.length > 0 ? (
        <ul
          id="results"
          ref={listRef}
          role="listbox"
          aria-label="Resultados"
          className="flex-1 overflow-y-auto p-2"
        >
          {results.map((r, i) => (
            <li
              key={r.id}
              id={`result-${r.id}`}
              data-index={i}
              role="option"
              aria-selected={i === selected}
              onMouseMove={() => setSelected(i)}
              onMouseDown={(e) => e.preventDefault()} // que el input no pierda el foco
              onClick={() => activate(r)}
              className={`flex items-center gap-3 rounded-md px-3 py-2 ${
                i === selected ? "bg-(--surface-selected)" : ""
              }`}
            >
              <span className="w-7 shrink-0 text-center text-xl" aria-hidden>
                {KIND_ICON[r.kind]}
              </span>
              <span className="min-w-0 flex-1">
                <span className="block truncate text-[15px]">{r.title}</span>
                <span className="block truncate text-xs text-(--text-muted)">
                  {r.subtitle}
                </span>
              </span>
              {i === selected && (
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

      <footer className="flex gap-4 border-t border-(--border) px-5 py-2 text-xs text-(--text-muted)">
        <span>↑↓ moverse</span>
        <span>↵ abrir</span>
        <span>Esc cerrar</span>
      </footer>
    </main>
  );
}

export default App;
