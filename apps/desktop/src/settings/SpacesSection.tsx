import { useCallback, useEffect, useState } from "react";
import { commands, events } from "../bindings";
import { SpaceIcon } from "../components/SpaceIcon";
import { arrayMove } from "@dnd-kit/sortable";
import { SortableList, useSortableItem } from "./Sortable";
import { SpaceEditor } from "./SpaceEditor";
import { emptyDraft, move, toDraft, type SpaceView } from "./draft";
import { Button, ErrorText } from "./ui";

/** Qué se está editando: un Espacio guardado (su id) o uno nuevo. */
type Selection = { kind: "space"; id: number } | { kind: "new" } | null;

/** ¿Hay que empezar creando un Espacio? (se abrió desde la barra con "Crear Espacio") */
const startsWithNew = new URLSearchParams(window.location.search).has("new");

/**
 * Un Espacio en la lista. Se coge por la fila entera para arrastrarlo (un clic sin
 * mover sigue seleccionándolo); Alt+↑/↓ lo mueve con el teclado.
 */
function SpaceRow({
  view,
  active,
  onSelect,
  onMove,
}: {
  view: SpaceView;
  active: boolean;
  onSelect: () => void;
  onMove: (delta: -1 | 1) => void;
}) {
  const { dragging, itemProps, handleProps } = useSortableItem(
    String(view.space.id),
  );
  return (
    <li
      {...itemProps}
      className={`flex items-center rounded-item ${
        dragging ? "relative z-10 bg-glass shadow-window" : ""
      }`}
    >
      <button
        type="button"
        {...handleProps}
        title="Arrastra para cambiar el orden (o Alt+↑/↓)"
        aria-current={active}
        onClick={onSelect}
        onKeyDown={(e) => {
          if (e.altKey && (e.key === "ArrowUp" || e.key === "ArrowDown")) {
            e.preventDefault();
            onMove(e.key === "ArrowUp" ? -1 : 1);
          }
        }}
        className="sift-option flex min-w-0 flex-1 touch-none items-center gap-2 px-3 py-1.5 text-left text-sm outline-none select-none hover:bg-hover focus-visible:ring-2 focus-visible:ring-accent"
      >
        <SpaceIcon
          icon={view.space.icon}
          autoIcon={view.autoIcon}
          name={view.space.name}
          size={20}
        />
        <span className="truncate">{view.space.name}</span>
      </button>
    </li>
  );
}

export function SpacesSection() {
  const [spaces, setSpaces] = useState<SpaceView[] | null>(null);
  const [selection, setSelection] = useState<Selection>(
    startsWithNew ? { kind: "new" } : null,
  );
  const [message, setMessage] = useState<{ ok: boolean; text: string } | null>(
    null,
  );
  // Cambia cada vez que se guarda/recarga, para que el editor empiece de cero.
  const [version, setVersion] = useState(0);

  const reload = useCallback(
    () =>
      commands.listSpaces().then((r) => {
        if (r.status === "ok") setSpaces(r.data);
        else setMessage({ ok: false, text: r.error.message });
        setVersion((v) => v + 1);
        return r.status === "ok" ? r.data : [];
      }),
    [],
  );

  useEffect(() => {
    void reload().then((list) => {
      // Sin selección previa, se abre el primero (si hay).
      setSelection(
        (sel) =>
          sel ?? (list[0] ? { kind: "space", id: list[0].space.id } : null),
      );
    });
    // "Crear Espacio" desde la barra con esta ventana ya abierta.
    const unlisten = events.newSpaceRequested.listen(() =>
      setSelection({ kind: "new" }),
    );
    return () => {
      void unlisten.then((stop) => stop());
    };
  }, [reload]);

  const [announcement, setAnnouncement] = useState("");

  /** Guarda un orden nuevo en SQLite (y lo pinta ya, sin esperar). */
  async function saveOrder(next: SpaceView[], moved: SpaceView) {
    setSpaces(next); // optimista: se ve al instante
    setAnnouncement(
      `«${moved.space.name}» movido a la posición ${next.indexOf(moved) + 1}`,
    );
    const r = await commands.reorderSpaces(next.map((v) => v.space.id));
    if (r.status === "error") {
      setMessage({ ok: false, text: r.error.message });
      void reload();
    }
  }

  function reorder(index: number, delta: -1 | 1) {
    if (!spaces) return;
    void saveOrder(move(spaces, index, delta), spaces[index]);
  }

  async function exportAll() {
    const r = await commands.exportSpaces();
    if (r.status === "error") setMessage({ ok: false, text: r.error.message });
    else if (r.data) setMessage({ ok: true, text: `Exportados a ${r.data}` });
  }

  async function importFile() {
    const r = await commands.importSpaces();
    if (r.status === "error")
      return setMessage({
        ok: false,
        text: `No se importó nada: ${r.error.message}`,
      });
    if (r.data === null) return;
    setMessage({
      ok: true,
      text: `${r.data} Espacio(s) importado(s). Los comandos pedirán confirmación la primera vez.`,
    });
    void reload();
  }

  const selected =
    selection?.kind === "space"
      ? spaces?.find((v) => v.space.id === selection.id)
      : undefined;

  return (
    <div className="flex h-full min-h-0">
      <aside className="flex w-64 shrink-0 flex-col border-r border-line">
        <p aria-live="polite" className="sr-only">
          {announcement}
        </p>
        <ul
          aria-label="Espacios"
          className="min-h-0 flex-1 space-y-0.5 overflow-y-auto px-2 pt-3"
        >
          {spaces?.length === 0 && (
            <li className="p-2 text-sm text-muted">Aún no hay Espacios.</li>
          )}
          <SortableList
            ids={(spaces ?? []).map((v) => String(v.space.id))}
            onMove={(from, to) => {
              if (spaces)
                void saveOrder(arrayMove(spaces, from, to), spaces[from]);
            }}
          >
            {spaces?.map((view, i) => (
              <SpaceRow
                key={view.space.id}
                view={view}
                active={
                  selection?.kind === "space" && selection.id === view.space.id
                }
                onSelect={() =>
                  setSelection({ kind: "space", id: view.space.id })
                }
                onMove={(delta) => reorder(i, delta)}
              />
            ))}
          </SortableList>
        </ul>
        <div className="space-y-2 border-t border-line p-3">
          {/* Fijo encima de Importar/Exportar: con muchos Espacios, siempre a mano. */}
          <Button
            className="w-full"
            aria-current={selection?.kind === "new"}
            onClick={() => setSelection({ kind: "new" })}
          >
            + Nuevo Espacio
          </Button>
          <div className="flex gap-2">
            <Button className="flex-1" onClick={() => void importFile()}>
              Importar…
            </Button>
            <Button
              className="flex-1"
              disabled={!spaces?.length}
              onClick={() => void exportAll()}
            >
              Exportar…
            </Button>
          </div>
          {message &&
            (message.ok ? (
              <p role="status" className="text-xs break-words text-success">
                {message.text}
              </p>
            ) : (
              <ErrorText>{message.text}</ErrorText>
            ))}
        </div>
      </aside>

      <section className="flex min-h-0 min-w-0 flex-1 flex-col">
        {selection?.kind === "new" && (
          <SpaceEditor
            key={`new-${version}`}
            id={null}
            saved={null}
            initial={emptyDraft()}
            onSaved={(id) =>
              void reload().then(() => setSelection({ kind: "space", id }))
            }
            onDeleted={() =>
              setSelection(
                spaces?.[0] ? { kind: "space", id: spaces[0].space.id } : null,
              )
            }
            onDuplicated={() => {}}
          />
        )}
        {selected && (
          <SpaceEditor
            key={`${selected.space.id}-${version}`}
            id={selected.space.id}
            saved={toDraft(selected.space)}
            initial={toDraft(selected.space)}
            autoIcon={selected.autoIcon}
            onSaved={() => void reload()}
            onDeleted={() =>
              void reload().then((list) =>
                setSelection(
                  list[0] ? { kind: "space", id: list[0].space.id } : null,
                ),
              )
            }
            onDuplicated={(id) =>
              void reload().then(() => setSelection({ kind: "space", id }))
            }
          />
        )}
        {!selection && spaces?.length === 0 && (
          <div className="flex h-full flex-col items-center justify-center gap-3 p-6 text-center">
            <p className="text-lg">Crea tu primer Espacio</p>
            <p className="max-w-sm text-sm text-muted">
              Por ejemplo «Bases de datos»: la carpeta de la asignatura, los
              apuntes, el campus virtual y `docker compose up` de las prácticas,
              todo con un Enter.
            </p>
            <Button
              variant="primary"
              onClick={() => setSelection({ kind: "new" })}
            >
              + Nuevo Espacio
            </Button>
          </div>
        )}
      </section>
    </div>
  );
}
