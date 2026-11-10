import { useEffect, useState } from "react";
import { commands } from "../bindings";
import { ActionEditor } from "./ActionEditor";
import { IconPicker } from "./IconPicker";
import {
  ACTION_LABELS,
  isDirty,
  move,
  newAction,
  parseAliases,
  type ActionKind,
  type Draft,
} from "./draft";
import { Button, ErrorText, Modal, TextInput } from "./ui";
import { arrayMove } from "@dnd-kit/sortable";
import { SortableList } from "./Sortable";
import { SpaceIcon } from "../components/SpaceIcon";

/** Ids solo para React: así, al reordenar, cada fila (y su estado) sigue a su acción. */
let nextUid = 0;
const uid = () => ++nextUid;

interface Props {
  /** `null` = Espacio nuevo, aún sin guardar. */
  id: number | null;
  saved: Draft | null;
  initial: Draft;
  autoIcon?: string | null;
  onSaved: (id: number) => void;
  onDeleted: () => void;
  onDuplicated: (id: number) => void;
}

export function SpaceEditor({
  id,
  saved,
  initial,
  autoIcon,
  onSaved,
  onDeleted,
  onDuplicated,
}: Props) {
  const [draft, setDraft] = useState<Draft>(initial);
  // Los alias se editan como texto ("bd, sql") y se convierten al guardar.
  const [aliasText, setAliasText] = useState(initial.aliases.join(", "));
  const [error, setError] = useState<string | null>(null);
  const [confirmDelete, setConfirmDelete] = useState(false);
  const [editingIcon, setEditingIcon] = useState(false);
  const [appIcons, setAppIcons] = useState<Record<string, string | null>>({});
  const [uids, setUids] = useState<number[]>(() => initial.actions.map(uid));

  const current: Draft = { ...draft, aliases: parseAliases(aliasText) };
  const dirty = isDirty(current, saved);

  // Iconos de las apps elegidas en las acciones.
  const appIds = draft.actions.flatMap((a) =>
    a.kind === "openApp" && a.appId ? [a.appId] : [],
  );
  const missing = appIds.filter((a) => !(a in appIcons));
  useEffect(() => {
    if (missing.length === 0) return;
    let alive = true;
    void commands.getIcons(missing.map((a) => `app:${a}`)).then((r) => {
      if (!alive || r.status !== "ok") return;
      setAppIcons((prev) => ({
        ...prev,
        ...Object.fromEntries(missing.map((a, i) => [a, r.data[i] ?? null])),
      }));
    });
    return () => {
      alive = false;
    };
    // `missing` cambia en cada render; con la lista unida en texto basta.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [missing.join("|")]);

  async function save() {
    const r =
      id === null
        ? await commands.createSpace(current)
        : await commands.updateSpace(id, current);
    if (r.status === "error") return setError(r.error.message);
    setError(null);
    onSaved(r.data.id);
  }

  async function duplicate() {
    if (id === null) return;
    const r = await commands.duplicateSpace(id);
    if (r.status === "error") setError(r.error.message);
    else onDuplicated(r.data.id);
  }

  async function remove() {
    if (id === null) return onDeleted();
    const r = await commands.deleteSpace(id);
    if (r.status === "error") setError(r.error.message);
    else onDeleted();
  }

  // Cada cambio en la lista de acciones mueve sus ids igual que las acciones.
  const setActions = (actions: Draft["actions"], ids: number[]) => {
    setDraft({ ...draft, actions });
    setUids(ids);
  };

  const [announcement, setAnnouncement] = useState("");
  // Arrastrar acciones: el orden nuevo entra en el borrador y se guarda con «Guardar».
  const moveAction = (from: number, to: number) =>
    setActions(arrayMove(draft.actions, from, to), arrayMove(uids, from, to));

  return (
    <form
      className="space-y-6"
      onSubmit={(e) => {
        e.preventDefault();
        void save();
      }}
      // Ctrl+S guarda, como en cualquier editor.
      onKeyDown={(e) => {
        if (e.ctrlKey && e.key.toLowerCase() === "s") {
          e.preventDefault();
          void save();
        }
      }}
    >
      {/*
        Tarjeta de contacto: avatar a la izquierda y, a su derecha, nombre y alias.
        El avatar mide exactamente lo que el bloque de campos (40 + 12 + 36 = 88 px, en la
        rejilla de 4 px), así sus bordes superior e inferior coinciden con los de los
        campos. La ayuda de los alias va debajo, a lo ancho, para no descuadrarlo.
      */}
      <div className="space-y-2">
        <div className="flex gap-4">
          <button
            type="button"
            onClick={() => setEditingIcon(true)}
            aria-label="Cambiar icono"
            title="Cambiar icono"
            className="group relative flex size-[88px] shrink-0 items-center justify-center rounded-window border border-line bg-hover outline-none hover:bg-selected focus-visible:ring-2 focus-visible:ring-accent"
          >
            <SpaceIcon
              icon={draft.icon}
              autoIcon={autoIcon}
              name={draft.name || "?"}
              size={48}
            />
            <span
              aria-hidden
              className="absolute inset-x-0 bottom-0 rounded-b-window bg-glass py-0.5 text-center text-[11px] text-muted opacity-0 backdrop-blur-xl group-hover:opacity-100 group-focus-visible:opacity-100"
            >
              Cambiar
            </span>
          </button>
          <div className="flex min-w-0 flex-1 flex-col gap-3">
            <TextInput
              autoFocus={id === null}
              aria-label="Nombre"
              placeholder="Nombre del Espacio"
              value={draft.name}
              maxLength={80}
              onChange={(e) =>
                setDraft({ ...draft, name: e.currentTarget.value })
              }
              className="h-10 text-lg font-semibold"
            />
            <label className="flex h-9 items-center gap-3">
              <span className="w-12 shrink-0 text-xs font-medium text-muted">
                Alias
              </span>
              <TextInput
                placeholder="bd, sql"
                value={aliasText}
                aria-describedby="alias-hint"
                onChange={(e) => setAliasText(e.currentTarget.value)}
                className="h-9"
              />
            </label>
          </div>
        </div>
        <p id="alias-hint" className="pl-[104px] text-xs text-muted">
          Alias: otras palabras que lo encuentran en la barra, separadas por
          comas.
        </p>
      </div>

      {editingIcon && (
        <Modal title="Icono del Espacio" onClose={() => setEditingIcon(false)}>
          <div className="max-h-[70vh] overflow-y-auto p-4">
            <IconPicker
              icon={draft.icon}
              autoIcon={autoIcon}
              name={draft.name}
              onChange={(icon) => setDraft({ ...draft, icon })}
            />
          </div>
          <footer className="flex justify-end border-t border-line px-4 py-3">
            <Button variant="primary" onClick={() => setEditingIcon(false)}>
              Listo
            </Button>
          </footer>
        </Modal>
      )}

      <section className="space-y-2">
        <h3 className="text-xs font-medium text-muted">Acciones, en orden</h3>
        {/* Anuncia a los lectores de pantalla los cambios de orden. */}
        <p aria-live="polite" className="sr-only">
          {announcement}
        </p>
        {draft.actions.length === 0 && (
          <p className="text-sm text-muted">
            Aún no hay acciones. Añade lo que quieres que se abra.
          </p>
        )}
        <SortableList ids={uids.map(String)} onMove={moveAction}>
          <ul className="space-y-2">
            {draft.actions.map((action, i) => (
              <ActionEditor
                key={uids[i]}
                action={action}
                index={i}
                count={draft.actions.length}
                appIcon={
                  action.kind === "openApp" ? appIcons[action.appId] : undefined
                }
                onChange={(a) =>
                  setActions(
                    draft.actions.map((old, j) => (j === i ? a : old)),
                    uids,
                  )
                }
                onMove={(delta) => {
                  setActions(
                    move(draft.actions, i, delta),
                    move(uids, i, delta),
                  );
                  setAnnouncement(
                    `Acción movida a la posición ${i + 1 + delta}`,
                  );
                }}
                sortableId={String(uids[i])}
                onRemove={() =>
                  setActions(
                    draft.actions.filter((_, j) => j !== i),
                    uids.filter((_, j) => j !== i),
                  )
                }
              />
            ))}
          </ul>
        </SortableList>
        <div className="flex flex-wrap gap-2">
          {(Object.keys(ACTION_LABELS) as ActionKind[]).map((kind) => (
            <Button
              key={kind}
              variant="ghost"
              onClick={() =>
                setActions(
                  [...draft.actions, newAction(kind)],
                  [...uids, uid()],
                )
              }
            >
              + {ACTION_LABELS[kind]}
            </Button>
          ))}
        </div>
      </section>

      {error && <ErrorText>{error}</ErrorText>}

      <footer className="sticky bottom-0 flex items-center gap-2 border-t border-line bg-glass py-3 backdrop-blur-xl">
        <Button type="submit" variant="primary" disabled={!dirty}>
          {id === null ? "Crear Espacio" : "Guardar"}
        </Button>
        {dirty && saved && (
          <Button
            variant="ghost"
            onClick={() => {
              setDraft(saved);
              setUids(saved.actions.map(uid));
              setAliasText(saved.aliases.join(", "));
            }}
          >
            Descartar cambios
          </Button>
        )}
        <span className="min-w-0 flex-1 truncate text-xs text-muted">
          {dirty ? "Cambios sin guardar · Ctrl+S" : "Guardado"}
        </span>
        {id !== null && (
          <Button onClick={() => void duplicate()}>Duplicar</Button>
        )}
        {confirmDelete ? (
          <>
            <span className="text-sm">¿Borrar «{saved?.name}»?</span>
            <Button variant="danger" onClick={() => void remove()}>
              Sí, borrar
            </Button>
            <Button variant="ghost" onClick={() => setConfirmDelete(false)}>
              No
            </Button>
          </>
        ) : (
          <Button
            variant="danger"
            onClick={() => (id === null ? onDeleted() : setConfirmDelete(true))}
          >
            {id === null ? "Cancelar" : "Borrar"}
          </Button>
        )}
      </footer>
    </form>
  );
}
