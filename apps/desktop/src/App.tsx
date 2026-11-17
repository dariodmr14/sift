// La barra: la caja de búsqueda, la lista (o un panel) y el pie con los atajos. Aquí solo
// se junta todo y se maneja el teclado; cada parte vive en `launcher/`.

import { useCallback, useMemo, useRef, useState } from "react";
import { commands } from "./bindings";
import { dismissLauncher, useLauncher } from "./useLauncher";
import { detectFilter, type TypeFilter } from "./launcher/filters";
import { buildItems, type Item } from "./launcher/items";
import { failedMode, type Mode } from "./launcher/mode";
import {
  EmptyState,
  openSettings,
  ResultPanel,
  RunningPanel,
} from "./launcher/panels";
import { ResultsList } from "./launcher/ResultsList";
import { moveSelection } from "./launcher/selection";
import { resultsAnnouncement } from "./launcher/announce";
import { useBarData } from "./launcher/useBarData";
import { useDebounced } from "./launcher/useDebounced";
import { useFileIcons, useFileSearch } from "./launcher/useFileSearch";
import { useSpaceRun } from "./launcher/useSpaceRun";
import { Glyph } from "./components/Glyph";
import { Shortcut } from "./components/Keycap";

function App() {
  const [query, setQuery] = useState("");
  const [selected, setSelected] = useState(0);
  const [mode, setMode] = useState<Mode>({ kind: "list" });
  // Filtro rápido activo («pdf …»): una etiqueta junto a la caja; Retroceso lo quita.
  const [filter, setFilter] = useState<TypeFilter | null>(null);
  const inputRef = useRef<HTMLInputElement>(null);
  const rootRef = useRef<HTMLElement>(null);

  const { spaces, recent, iconsHidden, refresh } = useBarData();
  const fileHits = useFileSearch(query, filter);
  const icons = useFileIcons([...fileHits, ...recent].map((h) => h.path));

  const dismiss = useCallback(() => dismissLauncher(rootRef.current), []);
  const reset = useCallback(() => {
    setQuery("");
    setFilter(null);
    setSelected(0);
    setMode({ kind: "list" });
  }, []);

  const { launch, cancel, shown, hidden } = useSpaceRun({
    setMode,
    onDone: dismiss,
    onResetHidden: reset,
  });

  useLauncher(
    rootRef,
    () => {
      // Si hay un Espacio en marcha o un resumen pendiente, se conserva.
      if (!shown()) reset();
      inputRef.current?.focus();
      void refresh();
    },
    // Al ocultarse se limpia ya, con la ventana invisible: al volver no se ve lo de antes.
    () => {
      if (hidden()) reset();
    },
  );

  const open = useCallback(
    (path: string, name: string, page: number | null) =>
      void commands.openFile(path, page).then((r) => {
        if (r.status === "error") setMode(failedMode(name, r.error.message));
        else dismiss();
      }),
    [dismiss],
  );

  const items = useMemo(
    () => buildItems({ query, filter, spaces, fileHits, recent, icons }),
    [query, filter, spaces, fileHits, recent, icons],
  );

  /** Lo que pasa al elegir una fila (Enter o clic). */
  const activate = (item: Item) => {
    const a = item.action;
    if (a.kind === "space") void launch(a.view);
    else if (a.kind === "file") open(a.path, a.name, a.page);
    else openSettings(a.newSpace);
  };

  const isEmpty = spaces.length === 0 && recent.length === 0 && !query.trim();

  function onKeyDown(e: React.KeyboardEvent<HTMLInputElement>) {
    const box = e.currentTarget;
    if (
      e.key === "Backspace" &&
      filter &&
      box.selectionStart === 0 &&
      box.selectionEnd === 0
    ) {
      e.preventDefault();
      setFilter(null);
      return;
    }
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
    if (mode.kind === "running") {
      if (e.key === "Escape") {
        e.preventDefault();
        cancel();
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
        else if (items[selected]) activate(items[selected]);
        break;
      case "Escape":
        e.preventDefault();
        dismiss();
        break;
    }
  }

  // Se anuncia 400 ms después del último cambio, para no leer cada pulsación.
  const announcement = useDebounced(
    resultsAnnouncement({
      query,
      filter,
      count: items.length,
      empty: !query.trim() && !filter,
    }),
    400,
  );

  const activeId =
    mode.kind === "list" && items[selected]
      ? `result-${items[selected].key}`
      : undefined;

  return (
    <main ref={rootRef} className="sift-from flex h-full flex-col bg-surface">
      <div className="flex items-center gap-2 border-b border-line px-5">
        {filter && (
          <button
            type="button"
            tabIndex={-1}
            onMouseDown={(e) => e.preventDefault()} // que la caja no pierda el foco
            onClick={() => setFilter(null)}
            title="Quitar el filtro (Retroceso al principio de la caja)"
            className="shrink-0 rounded-item bg-selected px-2 py-0.5 text-sm text-fg hover:bg-hover"
          >
            {filter.label}{" "}
            <Glyph name="close" className="ml-1 text-xs text-muted" />
          </button>
        )}
        <input
          ref={inputRef}
          autoFocus
          value={query}
          onChange={(e) => {
            const text = e.currentTarget.value;
            // «pdf » al principio: se convierte en la etiqueta y el resto es la búsqueda.
            const detected = filter ? null : detectFilter(text);
            if (detected) {
              setFilter(detected.filter);
              setQuery(detected.rest);
            } else {
              setQuery(text);
            }
            setSelected(0); // al escribir, la selección vuelve al primer resultado
            setMode({ kind: "list" });
          }}
          onKeyDown={onKeyDown}
          // Mientras se abre un Espacio, escribir no cambia nada (Esc sigue cancelando).
          readOnly={mode.kind === "running"}
          placeholder={filter ? `Buscar en tus ${filter.plural}` : "Buscar"}
          spellCheck={false}
          role="combobox"
          aria-label="Buscar"
          aria-expanded={mode.kind === "list" && items.length > 0}
          aria-controls="results"
          aria-activedescendant={activeId}
          className="min-w-0 flex-1 bg-transparent py-4 text-xl text-fg outline-none placeholder:text-muted"
        />
      </div>

      <div className="min-h-0 flex-1">
        {mode.kind === "running" ? (
          <RunningPanel
            title={mode.title}
            progress={mode.progress}
            steps={mode.steps}
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
          <ResultsList
            items={items}
            selected={selected}
            iconsHidden={iconsHidden}
            onHover={setSelected}
            onActivate={activate}
          />
        ) : (
          <p className="p-6 text-center text-muted">
            {filter && !query.trim()
              ? `Escribe qué buscar en tus ${filter.plural}`
              : filter
                ? `Sin ${filter.plural} para «${query}»`
                : `Sin resultados para «${query}»`}
          </p>
        )}
      </div>

      {/* Siempre montada: un lector de pantalla no suele leer una región recién puesta. */}
      <p role="status" aria-live="polite" className="sr-only">
        {mode.kind === "list" ? announcement : ""}
      </p>

      <footer className="flex items-center gap-4 border-t border-line px-5 py-2 text-xs text-muted">
        {mode.kind === "running" ? (
          <Hint keys="Esc">cancelar (lo ya abierto sigue abierto)</Hint>
        ) : mode.kind === "result" ? (
          <Hint keys="Esc">cerrar</Hint>
        ) : (
          <>
            <Hint keys="↑+↓">moverse</Hint>
            <Hint keys="enter">abrir</Hint>
            {items[selected]?.path ? (
              <>
                <Hint keys="Ctrl+enter">carpeta</Hint>
                <Hint keys="Ctrl+Shift+C">copiar ruta</Hint>
              </>
            ) : (
              <Hint keys="Ctrl+N">nuevo Espacio</Hint>
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
          className="ml-auto rounded-item p-1 leading-none hover:bg-hover hover:text-fg"
        >
          <Glyph name="settings" />
        </button>
      </footer>
    </main>
  );
}

/** Un atajo del pie: sus teclas y qué hace. */
function Hint({ keys, children }: { keys: string; children: string }) {
  return (
    <span className="flex items-center gap-1.5">
      <Shortcut keys={keys} />
      {children}
    </span>
  );
}

export default App;
