// El paso «Esperar a que…» del editor de Espacios (Fase 6, ADR 0012). Todo en lenguaje
// normal: qué se espera, cuánto como mucho y qué hacer si no ocurre.

import {
  WAIT_LABELS,
  withWaitKind,
  type WaitAction,
  type WaitKind,
} from "./draft";
import { Field, Select, TextInput } from "./ui";

/** ¿Se puede comprobar ahora, con «Probar»? (un comando o unos segundos, no) */
export function canCheckNow(action: WaitAction): boolean {
  return (
    action.until.kind === "webResponds" ||
    action.until.kind === "serviceAccepts"
  );
}

/** Número de un campo; vacío o no válido = 0 (al guardar se avisa). */
const toNumber = (text: string) => Number.parseInt(text, 10) || 0;

export function WaitEditor({
  action,
  onChange,
}: {
  action: WaitAction;
  onChange: (action: WaitAction) => void;
}) {
  const { until } = action;
  return (
    <div className="space-y-3">
      <div className="flex flex-wrap items-center gap-2 text-sm">
        <span>Esperar a que</span>
        <Select
          aria-label="A qué esperar"
          value={until.kind}
          onChange={(e) =>
            onChange(withWaitKind(action, e.currentTarget.value as WaitKind))
          }
        >
          {(Object.keys(WAIT_LABELS) as WaitKind[]).map((kind) => (
            <option key={kind} value={kind}>
              {WAIT_LABELS[kind]}
            </option>
          ))}
        </Select>
      </div>

      {until.kind === "webResponds" && (
        <Field
          label="Dirección"
          hint="Una web de tu equipo, como http://localhost:8080. Vale cualquier respuesta, aunque sea «página no encontrada»: significa que ya está en marcha."
        >
          <TextInput
            placeholder="http://localhost:8080"
            value={until.url}
            onChange={(e) =>
              onChange({
                ...action,
                until: { ...until, url: e.currentTarget.value },
              })
            }
          />
        </Field>
      )}

      {until.kind === "serviceAccepts" && (
        <Field
          label="Puerto"
          hint="El número por el que el servicio recibe conexiones. Suele venir en su configuración; por ejemplo, PostgreSQL usa el 5432, MySQL el 3306 y Redis el 6379."
        >
          <TextInput
            type="number"
            min={1}
            max={65535}
            placeholder="5432"
            className="w-32"
            value={until.port || ""}
            onChange={(e) =>
              onChange({
                ...action,
                until: { ...until, port: toNumber(e.currentTarget.value) },
              })
            }
          />
        </Field>
      )}

      {until.kind === "commandFinished" && (
        <p className="text-xs text-muted">
          Espera a que acabe el comando de justo antes (por ejemplo,{" "}
          <span className="font-mono">docker compose up -d</span>). Ese comando
          se ejecuta sin ventana y, si termina con error, verás sus últimas
          líneas.
        </p>
      )}

      {until.kind === "seconds" ? (
        <div className="flex items-center gap-2 text-sm">
          <TextInput
            type="number"
            min={1}
            max={600}
            aria-label="Segundos"
            className="w-24"
            value={until.secs || ""}
            onChange={(e) =>
              onChange({
                ...action,
                until: { ...until, secs: toNumber(e.currentTarget.value) },
              })
            }
          />
          <span>segundos</span>
        </div>
      ) : (
        <div className="flex flex-wrap items-center gap-2 text-sm">
          <span>
            Si no {until.kind === "commandFinished" ? "termina" : "ocurre"} en
          </span>
          <TextInput
            type="number"
            min={1}
            max={600}
            aria-label="Tiempo límite en segundos"
            className="w-24"
            value={action.timeoutSecs || ""}
            onChange={(e) =>
              onChange({
                ...action,
                timeoutSecs: toNumber(e.currentTarget.value),
              })
            }
          />
          <span>
            segundos
            {until.kind === "commandFinished" ? " o falla" : ""}:
          </span>
          <Select
            aria-label="Qué hacer si no ocurre"
            value={action.onTimeout}
            onChange={(e) =>
              onChange({
                ...action,
                onTimeout: e.currentTarget.value as WaitAction["onTimeout"],
              })
            }
          >
            <option value="stop">no seguir y avisarme</option>
            <option value="continue">seguir igualmente</option>
          </Select>
        </div>
      )}
    </div>
  );
}
