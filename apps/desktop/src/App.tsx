// La barra: la caja de búsqueda, la lista (o un panel) y el pie con los atajos. Aquí solo
// se junta todo y se maneja el teclado; cada parte vive en `launcher/`.

import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { commands } from "./bindings";
import { dismissLauncher, useLauncher } from "./useLauncher";
import { detectFilter, type TypeFilter } from "./launcher/filters";
import { buildItems, type Item } from "./launcher/items";
import { failedMode, panelKey, type Mode } from "./launcher/mode";
import {
  EmptyState,
  openSettings,
  ResultPanel,
  RunningPanel,
} from "./launcher/panels";
import {
  canPreview,
  PreviewPanel,
  NO_PREVIEW_TEXT,
  previewName,
} from "./launcher/PreviewPanel";
import { caretAtEnd, previewReason, reasonLabel } from "./launcher/preview";
import { ResultsList } from "./launcher/ResultsList";
import { moveSelection } from "./launcher/selection";
import { resultsAnnouncement } from "./launcher/announce";
import { useBarData } from "./launcher/useBarData";
import { useDebounced } from "./launcher/useDebounced";
import { useFileIcons, useFileSearch } from "./launcher/useFileSearch";
import { usePreview } from "./launcher/usePreview";
import { useSpaceRun } from "./launcher/useSpaceRun";
import { Glyph } from "./components/Glyph";
import { Shortcut } from "./components/Keycap";

const actionsCount = (n: number) => (n === 1 ? "1 acción" : `${n} acciones`);

function App() {
  const [query, setQuery] = useState("");
  const [selected, setSelected] = useState(0);
  const [mode, setMode] = useState<Mode>({ kind: "list" });
  // Filtro rápido activo («pdf …»): una etiqueta junto a la caja; Retroceso lo quita.
  const [filter, setFilter] = useState<TypeFilter | null>(null);
  // Vista previa del resultado seleccionado en lugar de la lista (Tab o →).
  const [previewOpen, setPreviewOpen] = useState(false);
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
    setPreviewOpen(false);
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

  const current: Item | undefined = items[selected];
  // Sin resultado seleccionado, o fuera de la lista, no hay vista previa.
  const showPreview = previewOpen && mode.kind === "list" && !!current;
  const previewState = usePreview(current?.hit, query, showPreview);

  const currentPath = current?.path;
  const isEmpty = spaces.length === 0 && recent.length === 0 && !query.trim();

  // Lo que hacen los atajos, para usarlo igual con el teclado que con el ratón (las pistas
  // del pie se pueden pulsar).
  const reveal = (path: string) =>
    void commands.revealFile(path).then(() => dismiss());
  const copyPath = (path: string) =>
    void navigator.clipboard.writeText(path).then(() => dismiss());
  const enter = () => {
    if (isEmpty) openSettings(true);
    else if (current) activate(current);
  };

  // Las teclas se escuchan en toda la ventana, no solo en la caja: si un clic deja el
  // foco en otro sitio (el texto de la vista previa, el fondo), los atajos siguen
  // funcionando y el foco vuelve a la caja antes de hacer nada.
  function onKeyDown(e: KeyboardEvent) {
    const box = inputRef.current;
    if (!box || e.defaultPrevented || e.isComposing) return;
    // La barra no tiene más paradas de Tab que la caja: si Tab moviera el foco, saldría
    // de la ventana y Windows la daría por perdida (y la barra se cerraría).
    if (e.key === "Tab") e.preventDefault();
    if (document.activeElement !== box) box.focus({ preventScroll: true });
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
    if (mode.kind === "list" && current?.path) {
      if (e.ctrlKey && e.key === "Enter") {
        e.preventDefault(); // Ctrl+↵: abrir la carpeta con el archivo seleccionado
        reveal(current.path);
        return;
      }
      if (e.ctrlKey && e.shiftKey && e.code === "KeyC") {
        e.preventDefault(); // Ctrl+Shift+C: copiar la ruta
        copyPath(current.path);
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
    if (mode.kind === "list") {
      const plain = !e.ctrlKey && !e.altKey && !e.metaKey;
      if (plain && e.key === "Tab") {
        if (showPreview) setPreviewOpen(false);
        else if (canPreview(current) && !e.shiftKey) setPreviewOpen(true);
        return;
      }
      if (
        plain &&
        !e.shiftKey &&
        e.key === "ArrowRight" &&
        !showPreview &&
        canPreview(current) &&
        caretAtEnd(box.value, box.selectionStart, box.selectionEnd)
      ) {
        e.preventDefault();
        setPreviewOpen(true);
        return;
      }
      if (showPreview && (e.key === "Escape" || e.key === "ArrowLeft")) {
        e.preventDefault(); // vuelve a la lista; Esc no cierra la barra
        setPreviewOpen(false);
        return;
      }
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
        enter();
        break;
      case "Escape":
        e.preventDefault();
        dismiss();
        break;
    }
  }

  // Siempre la última versión de `onKeyDown` (lee el estado del render actual), con un
  // solo listener en la ventana para toda la vida de la barra.
  const keyHandler = useRef(onKeyDown);
  useEffect(() => {
    keyHandler.current = onKeyDown;
  });
  useEffect(() => {
    const listener = (e: KeyboardEvent) => keyHandler.current(e);
    window.addEventListener("keydown", listener);
    return () => window.removeEventListener("keydown", listener);
  }, []);

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
    mode.kind === "list" && !showPreview && current
      ? `result-${current.key}`
      : undefined;
  const previewAnnouncement =
    showPreview && current
      ? `Vista previa de ${previewName(current, current.hit)}. ${
          current.hit
            ? `${reasonLabel(previewReason(current.hit))}.`
            : current.action.kind === "space"
              ? `${actionsCount(current.action.view.space.actions.length)}. Enter para abrir el Espacio.`
              : NO_PREVIEW_TEXT
        }`
      : "";

  return (
    <main
      ref={rootRef}
      // Un clic en cualquier parte de la barra no le quita el foco a la caja (salvo para
      // seleccionar el texto de la vista previa, que se puede copiar).
      onMouseDown={(e) => {
        const target = e.target as HTMLElement;
        if (target !== inputRef.current && !target.closest("[data-selectable]"))
          e.preventDefault();
      }}
      className="sift-from flex h-full flex-col bg-surface"
    >
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
            setPreviewOpen(false);
            setMode({ kind: "list" });
          }}
          // Mientras se abre un Espacio, escribir no cambia nada (Esc sigue cancelando).
          readOnly={mode.kind === "running"}
          placeholder={filter ? `Buscar en tus ${filter.plural}` : "Buscar"}
          spellCheck={false}
          role="combobox"
          aria-label="Buscar"
          aria-expanded={
            mode.kind === "list" && !showPreview && items.length > 0
          }
          aria-controls="results"
          aria-activedescendant={activeId}
          className="min-w-0 flex-1 bg-transparent py-4 text-xl text-fg outline-none placeholder:text-muted"
        />
      </div>

      <div
        key={panelKey({
          mode: mode.kind,
          isEmpty,
          hasItems: items.length > 0,
          preview: showPreview,
        })}
        className="sift-swap min-h-0 flex-1"
      >
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
        ) : showPreview && current ? (
          <PreviewPanel
            item={current}
            hit={current.hit}
            state={previewState}
            icon={iconsHidden ? undefined : current.icon}
            onBack={() => setPreviewOpen(false)}
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
        {mode.kind === "list"
          ? showPreview
            ? previewAnnouncement
            : announcement
          : ""}
      </p>

      <footer className="flex items-center gap-4 border-t border-line px-5 py-2 text-xs text-muted">
        {mode.kind === "running" ? (
          <Hint keys="Esc" onClick={cancel}>
            Cancelar (lo ya abierto sigue abierto)
          </Hint>
        ) : mode.kind === "result" ? (
          <Hint keys="Esc" onClick={dismiss}>
            Cerrar
          </Hint>
        ) : showPreview ? (
          <>
            <Hint keys="↑+↓">Cambiar</Hint>
            <Hint keys="enter" onClick={enter}>
              {current?.action.kind === "space" ? "Abrir el Espacio" : "Abrir"}
            </Hint>
            <Hint keys="Esc" onClick={() => setPreviewOpen(false)}>
              Volver
            </Hint>
            {currentPath && (
              <Hint keys="Ctrl+enter" onClick={() => reveal(currentPath)}>
                Carpeta
              </Hint>
            )}
          </>
        ) : (
          <>
            <Hint keys="↑+↓">Moverse</Hint>
            {(isEmpty || current) && (
              <Hint keys="enter" onClick={enter}>
                Abrir
              </Hint>
            )}
            {canPreview(current) && (
              <Hint keys="Tab" onClick={() => setPreviewOpen(true)}>
                Vista previa
              </Hint>
            )}
            {currentPath ? (
              <>
                <Hint keys="Ctrl+enter" onClick={() => reveal(currentPath)}>
                  Carpeta
                </Hint>
                <Hint keys="Ctrl+Shift+C" onClick={() => copyPath(currentPath)}>
                  Copiar ruta
                </Hint>
              </>
            ) : (
              <Hint keys="Ctrl+N" onClick={() => openSettings(true)}>
                Nuevo Espacio
              </Hint>
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

/**
 * Un atajo del pie: sus teclas y qué hace. Con `onClick` también se puede pulsar con el
 * ratón (sin quitarle el foco a la caja); sin él, solo informa («↑↓ Moverse»).
 */
function Hint({
  keys,
  onClick,
  children,
}: {
  keys: string;
  onClick?: () => void;
  children: string;
}) {
  if (!onClick)
    return (
      <span className="flex items-center gap-1.5">
        <Shortcut keys={keys} />
        {children}
      </span>
    );
  return (
    <button
      type="button"
      tabIndex={-1}
      onMouseDown={(e) => e.preventDefault()}
      onClick={onClick}
      // El nombre es lo que hace; las teclas van en la descripción emergente.
      aria-label={children}
      title={`${children} (${keys.replace("enter", "Intro")})`}
      className="-mx-1 flex min-h-6 items-center gap-1.5 rounded-item px-1 hover:bg-hover hover:text-fg"
    >
      <Shortcut keys={keys} />
      {children}
    </button>
  );
}

export default App;
