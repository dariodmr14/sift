import { useState } from "react";
import { commands, type InstalledApp } from "../bindings";
import { ACTION_LABELS, argsFromText, argsToText, type Action } from "./draft";
import { AppPicker } from "./AppPicker";
import { Button, ErrorText, Field, TextInput } from "./ui";

type TestState =
  | { kind: "idle" }
  | { kind: "running" }
  | { kind: "ok" }
  | { kind: "error"; message: string }
  /** Es un comando que nunca se ha ejecutado: se muestra y se pide confirmar. */
  | { kind: "confirm"; display: string };

interface Props {
  action: Action;
  index: number;
  count: number;
  appIcon?: string | null;
  onChange: (action: Action) => void;
  onMove: (delta: -1 | 1) => void;
  onRemove: () => void;
}

export function ActionEditor({
  action,
  index,
  count,
  appIcon,
  onChange,
  onMove,
  onRemove,
}: Props) {
  const [test, setTest] = useState<TestState>({ kind: "idle" });
  const [picking, setPicking] = useState(false);

  // Si cambias la acción, el resultado de la prueba anterior ya no vale.
  const change = (next: Action) => {
    setTest({ kind: "idle" });
    onChange(next);
  };

  async function runTest(approve: boolean) {
    setTest({ kind: "running" });
    const r = await commands.testAction(action, approve);
    if (r.status === "error")
      return setTest({ kind: "error", message: r.error.message });
    if (r.data.status === "needsApproval") {
      return setTest({
        kind: "confirm",
        display: r.data.commands.map((c) => c.display).join("\n"),
      });
    }
    const failed = r.data.outcomes.find((o) => o.error);
    setTest(
      failed ? { kind: "error", message: failed.error! } : { kind: "ok" },
    );
  }

  async function pickPath(
    kind: "file" | "folder",
    apply: (path: string) => void,
  ) {
    const r = await commands.pickPath(kind);
    if (r.status === "ok" && r.data) apply(r.data);
  }

  return (
    <li className="space-y-3 rounded-window border border-line p-3">
      <div className="flex items-center gap-2">
        <span className="text-xs text-muted">{index + 1}.</span>
        <span className="flex-1 text-sm font-medium">
          {ACTION_LABELS[action.kind]}
        </span>
        <Button
          variant="ghost"
          aria-label="Subir acción"
          disabled={index === 0}
          onClick={() => onMove(-1)}
        >
          ↑
        </Button>
        <Button
          variant="ghost"
          aria-label="Bajar acción"
          disabled={index === count - 1}
          onClick={() => onMove(1)}
        >
          ↓
        </Button>
        <Button
          onClick={() => void runTest(false)}
          disabled={test.kind === "running"}
        >
          {test.kind === "running" ? "Probando…" : "Probar"}
        </Button>
        <Button variant="danger" aria-label="Quitar acción" onClick={onRemove}>
          ✕
        </Button>
      </div>

      {(action.kind === "openFolder" || action.kind === "openFile") && (
        <div className="flex gap-2">
          <TextInput
            aria-label="Ruta"
            placeholder={
              action.kind === "openFolder"
                ? "C:\\Users\\…\\Carpeta"
                : "C:\\Users\\…\\archivo.pdf"
            }
            value={action.path}
            onChange={(e) => change({ ...action, path: e.currentTarget.value })}
          />
          <Button
            onClick={() =>
              void pickPath(
                action.kind === "openFolder" ? "folder" : "file",
                (path) => change({ ...action, path }),
              )
            }
          >
            Elegir…
          </Button>
        </div>
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
              {appIcon ? <img src={appIcon} alt="" className="size-6" /> : null}
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

      {action.kind === "runCommand" && (
        <div className="grid grid-cols-2 gap-3">
          <Field
            label="Programa"
            hint="Un .exe del PATH (docker, git) o la ruta completa. Los .cmd con extensión: code.cmd"
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
            <div className="flex gap-2">
              <TextInput
                placeholder="C:\Users\…"
                value={action.cwd ?? ""}
                onChange={(e) =>
                  change({ ...action, cwd: e.currentTarget.value || null })
                }
              />
              <Button
                onClick={() =>
                  void pickPath("folder", (cwd) => change({ ...action, cwd }))
                }
              >
                Elegir…
              </Button>
            </div>
          </Field>
          <div className="col-span-2">
            <Field
              label="Argumentos, uno por línea"
              hint="Cada línea llega al programa tal cual, sin pasar por una shell: no hacen falta comillas."
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
                className="w-full rounded-item border border-line bg-hover px-3 py-1.5 font-mono text-sm outline-none focus:border-accent"
              />
            </Field>
          </div>
        </div>
      )}

      {test.kind === "ok" && <p className="text-sm text-success">Funciona.</p>}
      {test.kind === "error" && <ErrorText>{test.message}</ErrorText>}
      {test.kind === "confirm" && (
        <div
          role="alertdialog"
          className="space-y-2 rounded-item bg-selected p-3 text-sm"
        >
          <p>Este comando se va a ejecutar por primera vez:</p>
          <pre className="font-mono text-[13px] whitespace-pre-wrap select-text">
            {test.display}
          </pre>
          <div className="flex gap-2">
            <Button variant="primary" onClick={() => void runTest(true)}>
              Ejecutar
            </Button>
            <Button variant="ghost" onClick={() => setTest({ kind: "idle" })}>
              Cancelar
            </Button>
          </div>
        </div>
      )}
    </li>
  );
}
