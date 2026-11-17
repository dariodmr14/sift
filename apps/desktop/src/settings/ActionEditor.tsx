import { useState } from "react";
import { commands, type InstalledApp } from "../bindings";
import { ACTION_LABELS, argsFromText, argsToText, type Action } from "./draft";
import { AppPicker } from "./AppPicker";
import { PathField } from "./PathField";
import { PlacementEditor } from "./PlacementEditor";
import { useSortableItem } from "./Sortable";
import { Button, ErrorText, Field, TextInput } from "./ui";
import { canCheckNow, WaitEditor } from "./WaitEditor";
import { Glyph } from "../components/Glyph";

type TestState =
  | { kind: "idle" }
  | { kind: "running" }
  /** Fue bien; en una espera, lo que se ha visto («responde ahora mismo»). */
  | { kind: "ok"; message?: string }
  | { kind: "error"; message: string }
  /** Pedía confirmar un comando nuevo y en el diálogo se eligió «Cancelar». */
  | { kind: "declined" };

interface Props {
  action: Action;
  index: number;
  count: number;
  appIcon?: string | null;
  onChange: (action: Action) => void;
  onMove: (delta: -1 | 1) => void;
  onRemove: () => void;
  /** Id estable de la tarjeta para arrastrarla (dentro de una `SortableList`). */
  sortableId: string;
}

export function ActionEditor({
  action,
  index,
  count,
  appIcon,
  onChange,
  onMove,
  onRemove,
  sortableId,
}: Props) {
  const [test, setTest] = useState<TestState>({ kind: "idle" });
  const { dragging, itemProps, handleProps } = useSortableItem(sortableId);
  const [picking, setPicking] = useState(false);

  // Si cambias la acción, el resultado de la prueba anterior ya no vale.
  const change = (next: Action) => {
    setTest({ kind: "idle" });
    onChange(next);
  };

  async function runTest() {
    setTest({ kind: "running" });
    // Una espera no se ejecuta: se comprueba una vez, ahora, sin lanzar el Espacio.
    if (action.kind === "wait") {
      const r = await commands.checkWait(action);
      return setTest(
        r.status === "ok"
          ? { kind: "ok", message: r.data }
          : { kind: "error", message: r.error.message },
      );
    }
    const r = await commands.testAction(action);
    if (r.status === "error")
      return setTest({ kind: "error", message: r.error.message });
    if (r.data.status === "declined") return setTest({ kind: "declined" });
    const failed = r.data.outcomes.find((o) => o.error);
    setTest(
      failed ? { kind: "error", message: failed.error! } : { kind: "ok" },
    );
  }

  return (
    <li
      {...itemProps}
      // Alt+↑/↓ desde cualquier sitio de la tarjeta: la alternativa con teclado a arrastrar.
      onKeyDown={(e) => {
        if (e.altKey && (e.key === "ArrowUp" || e.key === "ArrowDown")) {
          e.preventDefault();
          const delta = e.key === "ArrowUp" ? -1 : 1;
          if ((delta < 0 && index > 0) || (delta > 0 && index < count - 1))
            onMove(delta);
        }
      }}
      // La que va en la mano se levanta (vidrio y sombra) y no deja seleccionar nada.
      className={`space-y-3 rounded-window border border-line p-3 ${
        dragging ? "relative z-10 bg-glass shadow-window select-none" : ""
      }`}
    >
      <div className="flex items-center gap-2">
        <button
          type="button"
          {...handleProps}
          aria-label={`Mover acción ${index + 1}: arrastra, o Alt+↑/↓`}
          title="Arrastra para reordenar (o Alt+↑/↓)"
          className="cursor-grab touch-none rounded-item px-1 text-muted outline-none select-none hover:bg-hover hover:text-fg focus-visible:ring-2 focus-visible:ring-accent-ink active:cursor-grabbing"
        >
          <Glyph name="gripper" />
        </button>
        <span className="text-xs text-muted">{index + 1}.</span>
        <span className="flex-1 text-sm font-medium">
          {ACTION_LABELS[action.kind]}
        </span>
        <Button
          onClick={() => void runTest()}
          disabled={
            test.kind === "running" ||
            (action.kind === "wait" && !canCheckNow(action))
          }
          title={
            action.kind === "wait"
              ? canCheckNow(action)
                ? "Comprueba una vez si ya se cumple, sin abrir el Espacio"
                : "Esta espera no tiene nada que comprobar por adelantado"
              : undefined
          }
        >
          {test.kind === "running" ? "Probando…" : "Probar"}
        </Button>
        <Button
          variant="danger"
          aria-label="Quitar acción"
          title="Quitar acción"
          onClick={onRemove}
        >
          <Glyph name="delete" />
        </Button>
      </div>

      {(action.kind === "openFolder" || action.kind === "openFile") && (
        <PathField
          label="Ruta"
          kind={action.kind === "openFolder" ? "folder" : "file"}
          placeholder={
            action.kind === "openFolder"
              ? "Escribe su nombre o C:\\Users\\…\\Carpeta"
              : "Escribe su nombre o C:\\Users\\…\\archivo.pdf"
          }
          value={action.path}
          onChange={(path) => change({ ...action, path })}
        />
      )}

      {action.kind === "openUrl" && (
        <TextInput
          aria-label="URL"
          placeholder="https://…"
          value={action.url}
          onChange={(e) => change({ ...action, url: e.currentTarget.value })}
        />
      )}

      {action.kind === "openApp" && (
        <div className="flex items-center gap-3">
          {action.appId ? (
            <>
              {appIcon ? (
                <img
                  src={appIcon}
                  alt=""
                  className="size-6"
                  draggable={false}
                />
              ) : null}
              <span className="flex-1 text-sm">{action.name}</span>
            </>
          ) : (
            <span className="flex-1 text-sm text-muted">
              Ninguna aplicación elegida
            </span>
          )}
          <Button onClick={() => setPicking(true)}>
            {action.appId ? "Cambiar…" : "Elegir aplicación…"}
          </Button>
          {picking && (
            <AppPicker
              onClose={() => setPicking(false)}
              onPick={(app: InstalledApp) => {
                setPicking(false);
                change({ kind: "openApp", appId: app.id, name: app.name });
              }}
            />
          )}
        </div>
      )}

      {(action.kind === "openFolder" ||
        action.kind === "openFile" ||
        action.kind === "openUrl" ||
        action.kind === "openApp") && (
        <PlacementEditor
          value={action.place}
          onChange={(place) => change({ ...action, place: place ?? undefined })}
        />
      )}

      {action.kind === "runCommand" && (
        <div className="grid grid-cols-2 gap-3">
          <Field
            label="Programa"
            hint="Su nombre, si Windows ya sabe dónde está (como git), o su ruta completa. Si termina en .cmd o .bat, escríbelo también: code.cmd"
          >
            <TextInput
              placeholder="docker"
              value={action.program}
              onChange={(e) =>
                change({ ...action, program: e.currentTarget.value })
              }
            />
          </Field>
          <Field label="Carpeta de trabajo (opcional)">
            <PathField
              label="Carpeta de trabajo"
              kind="folder"
              placeholder="Su nombre o C:\Users\…"
              value={action.cwd ?? ""}
              onChange={(cwd) => change({ ...action, cwd: cwd || null })}
            />
          </Field>
          <div className="col-span-2">
            <Field
              label="Argumentos, uno por línea"
              hint="Cada línea llega al programa tal cual, como una sola pieza: no hacen falta comillas."
            >
              <textarea
                spellCheck={false}
                rows={Math.max(2, action.args.length + 1)}
                placeholder={"compose\nup"}
                value={argsToText(action.args)}
                onChange={(e) =>
                  change({
                    ...action,
                    args: argsFromText(e.currentTarget.value),
                  })
                }
                className="w-full rounded-item border border-line bg-hover px-3 py-1.5 font-mono text-sm outline-none focus:border-accent-ink"
              />
            </Field>
          </div>
        </div>
      )}

      {action.kind === "wait" && (
        <WaitEditor action={action} onChange={change} />
      )}

      {test.kind === "ok" && (
        <p className="text-sm text-success">{test.message ?? "Funciona."}</p>
      )}
      {test.kind === "error" && <ErrorText>{test.message}</ErrorText>}
      {test.kind === "declined" && (
        <p className="text-sm text-muted">
          No se ejecutó: cancelaste la confirmación.
        </p>
      )}
    </li>
  );
}
