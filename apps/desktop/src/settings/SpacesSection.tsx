import { useCallback, useEffect, useState } from "react";
import { commands, events } from "../bindings";
import { SpaceIcon } from "../components/SpaceIcon";
import { SpaceEditor } from "./SpaceEditor";
import { emptyDraft, move, toDraft, type SpaceView } from "./draft";
import { Button, ErrorText } from "./ui";

/** Qué se está editando: un Espacio guardado (su id) o uno nuevo. */
type Selection = { kind: "space"; id: number } | { kind: "new" } | null;

/** ¿Hay que empezar creando un Espacio? (se abrió desde la barra con "Crear Espacio") */
const startsWithNew = new URLSearchParams(window.location.search).has("new");

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

  async function reorder(index: number, delta: -1 | 1) {
    if (!spaces) return;
    const next = move(spaces, index, delta);
    setSpaces(next); // optimista: se ve al instante
    const r = await commands.reorderSpaces(next.map((v) => v.space.id));
    if (r.status === "error") {
      setMessage({ ok: false, text: r.error.message });
      void reload();
    }
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
        <div className="flex gap-2 p-3">
          <Button
            variant="primary"
            className="flex-1"
            onClick={() => setSelection({ kind: "new" })}
          >
            + Nuevo Espacio
          </Button>
        </div>
        <ul
          aria-label="Espacios"
          className="min-h-0 flex-1 space-y-0.5 overflow-y-auto px-2"
        >
          {spaces?.length === 0 && (
            <li className="p-2 text-sm text-muted">Aún no hay Espacios.</li>
          )}
          {spaces?.map((view, i) => {
            const active =
              selection?.kind === "space" && selection.id === view.space.id;
            return (
              <li key={view.space.id} className="group flex items-center">
                <button
                  type="button"
                  aria-current={active}
                  onClick={() =>
                    setSelection({ kind: "space", id: view.space.id })
                  }
                  // Alt+↑/↓ reordena con el teclado.
                  onKeyDown={(e) => {
                    if (
                      e.altKey &&
                      (e.key === "ArrowUp" || e.key === "ArrowDown")
                    ) {
                      e.preventDefault();
                      void reorder(i, e.key === "ArrowUp" ? -1 : 1);
                    }
                  }}
                  className="sift-option flex min-w-0 flex-1 items-center gap-2 px-3 py-1.5 text-left text-sm outline-none hover:bg-hover focus-visible:ring-2 focus-visible:ring-accent"
                >
                  <SpaceIcon
                    icon={view.space.icon}
                    autoIcon={view.autoIcon}
                    name={view.space.name}
                    size={20}
                  />
                  <span className="truncate">{view.space.name}</span>
                </button>
                <span className="flex opacity-0 group-hover:opacity-100 group-focus-within:opacity-100">
                  <Button
                    variant="ghost"
                    className="px-1.5"
                    aria-label={`Subir ${view.space.name}`}
                    title="Subir (Alt+↑)"
                    disabled={i === 0}
                    onClick={() => void reorder(i, -1)}
                  >
                    ↑
                  </Button>
                  <Button
                    variant="ghost"
                    className="px-1.5"
                    aria-label={`Bajar ${view.space.name}`}
                    title="Bajar (Alt+↓)"
                    disabled={i === spaces.length - 1}
                    onClick={() => void reorder(i, 1)}
                  >
                    ↓
                  </Button>
                </span>
              </li>
            );
          })}
        </ul>
        <div className="space-y-2 border-t border-line p-3">
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

      <section className="min-w-0 flex-1 overflow-y-auto px-6 pt-6">
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
          <div className="flex h-full flex-col items-center justify-center gap-3 text-center">
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
