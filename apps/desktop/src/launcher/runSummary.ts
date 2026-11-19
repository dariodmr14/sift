// Qué contar al acabar de abrir un Espacio (lógica pura, para probarla). ADR 0012.

/** Lo que llega de Rust por cada acción. */
export interface Outcome {
  index: number;
  label: string;
  error: string | null;
  /** `false` si no llegó a ejecutarse (se paró antes o se canceló). */
  ran: boolean;
  /** Un aviso que no es un fallo («se abrió, pero no encontré su ventana…»). */
  note?: string | null;
}

export type LineState = "ok" | "warning" | "failed" | "cancelled" | "notRun";

export interface SummaryLine {
  label: string;
  state: LineState;
  /** El motivo, si falló (puede tener varias líneas: la salida de un comando). */
  detail?: string;
}

export interface Summary {
  headline: string;
  lines: SummaryLine[];
  /** `false` si todo fue bien: entonces la barra se cierra sin enseñar nada. */
  needsAttention: boolean;
}

function lineFor(o: Outcome): SummaryLine {
  if (o.ran && !o.error && o.note)
    return { label: o.label, state: "warning", detail: o.note };
  if (o.ran && !o.error) return { label: o.label, state: "ok" };
  if (o.ran) return { label: o.label, state: "failed", detail: o.error! };
  if (o.error) return { label: o.label, state: "cancelled", detail: o.error };
  return { label: o.label, state: "notRun" };
}

/** Resumen de lo que se ejecutó y lo que no. `cancelled`: se pulsó Esc a mitad. */
export function summarize(outcomes: Outcome[], cancelled: boolean): Summary {
  const lines = outcomes.map(lineFor);
  const failed = lines.some((l) => l.state === "failed");
  const stopped = lines.some((l) => l.state === "notRun");
  const kept = "lo que ya se abrió sigue abierto";
  if (cancelled) {
    return {
      headline: `Cancelado: ${kept}`,
      lines,
      needsAttention: true,
    };
  }
  if (stopped) {
    const at = outcomes.find((o) => o.ran && o.error)?.index;
    return {
      headline: `Se paró${at ? ` en el paso ${at}` : ""}: ${kept}`,
      lines,
      needsAttention: true,
    };
  }
  const warned = lines.some((l) => l.state === "warning");
  return {
    headline: failed
      ? "Algo no se pudo abrir"
      : warned
        ? "Abierto, con avisos"
        : "Abierto",
    lines,
    needsAttention: failed || warned,
  };
}

/** «Paso 2 de 3 · Esperando a que responda …… 12 s» */
/**
 * Un paso ya lanzado cuya ventana aún se está colocando: «Abrir Eclipse» → «Eclipse
 * abierto · colocando…».
 */
export function placingText(label: string): string {
  const opened = label
    .replace(/^Abrir carpeta (.*)$/, "Carpeta $1 abierta")
    .replace(/^Abrir (\w+:\/\/.*)$/, "$1 abierta")
    .replace(/^Abrir (.*)$/, "$1 abierto");
  return `${opened} · colocando…`;
}

export function progressText(p: {
  step: number;
  total: number;
  label: string;
  waited: number | null;
}): string {
  const base = `Paso ${p.step} de ${p.total} · ${p.label}`;
  if (p.waited === null) return `${base}…`;
  // «Esperar a que…» → «Esperando a que…» mientras espera.
  const label = p.label.replace(/^Esperar\b/, "Esperando");
  return `Paso ${p.step} de ${p.total} · ${label}… ${p.waited} s`;
}
