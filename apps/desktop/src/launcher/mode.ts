// En qué está la barra: la lista, un Espacio abriéndose o el resumen de lo que pasó.

import type { commands } from "../bindings";
import type { SpaceProgress } from "./panels";
import type { SummaryLine } from "./runSummary";

type Ok<T> = Extract<T, { status: "ok" }> extends { data: infer D } ? D : never;
export type SpaceView = Ok<
  Awaited<ReturnType<typeof commands.listSpaces>>
>[number];

export type Mode =
  | { kind: "list" }
  /** Un Espacio en marcha (con esperas puede tardar): por qué paso va. Esc cancela. */
  | {
      kind: "running";
      title: string;
      progress: SpaceProgress | null;
      cancelling: boolean;
    }
  /** Algo falló, se paró o se canceló: qué se ejecutó y qué no. */
  | { kind: "result"; title: string; headline: string; lines: SummaryLine[] };

/** El resumen de algo que no se pudo abrir (un archivo, un Espacio). */
export function failedMode(title: string, detail: string): Mode {
  return {
    kind: "result",
    title,
    headline: "No se pudo abrir",
    lines: [{ label: title, state: "failed", detail }],
  };
}
