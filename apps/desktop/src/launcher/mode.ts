// En qué está la barra: la lista, un Espacio abriéndose o el resumen de lo que pasó.

import type { commands, PlacementState } from "../bindings";
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
      /** Los pasos ya empezados, en orden (el último es el que está en marcha). */
      steps: RunStep[];
      cancelling: boolean;
    }
  /** Algo falló, se paró o se canceló: qué se ejecutó y qué no. */
  | { kind: "result"; title: string; headline: string; lines: SummaryLine[] };

export interface RunStep {
  step: number;
  label: string;
  /** Si hay que colocar su ventana: en qué va (llega en segundo plano). */
  placement?: PlacementState;
}

/** Apunta un aviso de progreso: un paso nuevo se añade; el mismo (una espera) no. */
export function addStep(steps: RunStep[], p: SpaceProgress): RunStep[] {
  const last = steps[steps.length - 1];
  if (last?.step === p.step) return steps;
  return [...steps, { step: p.step, label: p.label }];
}

/** Apunta en qué va la colocación de la ventana de un paso ya lanzado. */
export function setPlacement(
  steps: RunStep[],
  p: { step: number; state: PlacementState },
): RunStep[] {
  return steps.map((s) =>
    s.step === p.step ? { ...s, placement: p.state } : s,
  );
}

/** ¿Queda alguna ventana por colocar? */
export function placing(steps: RunStep[]): boolean {
  return steps.some((s) => s.placement === "placing");
}

/** Qué ocupa la barra bajo la caja; si cambia, el panel entra con una animación breve. */
export function panelKey(args: {
  mode: Mode["kind"];
  isEmpty: boolean;
  hasItems: boolean;
  /** La vista previa del resultado seleccionado ocupa el lugar de la lista. */
  preview: boolean;
}): "running" | "result" | "preview" | "empty" | "list" | "none" {
  if (args.mode === "running" || args.mode === "result") return args.mode;
  if (args.preview) return "preview";
  if (args.isEmpty) return "empty";
  return args.hasItems ? "list" : "none";
}

/** El resumen de algo que no se pudo abrir (un archivo, un Espacio). */
export function failedMode(title: string, detail: string): Mode {
  return {
    kind: "result",
    title,
    headline: "No se pudo abrir",
    lines: [{ label: title, state: "failed", detail }],
  };
}
