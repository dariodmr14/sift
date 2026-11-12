// Zonas y pantallas para colocar ventanas (Fase 6.5, ADR 0014). Lógica pura.

import type { MonitorChoice, MonitorInfo, Zone } from "../bindings";

export interface ZoneInfo {
  id: Zone;
  /** Lo que se lee («Mitad izquierda»). */
  label: string;
  /** Dónde está, en sextos de la pantalla: [izquierda, arriba, ancho, alto]. El mismo
   *  reparto que en Rust (`Zone::sixths`), para dibujar la miniatura. */
  sixths: [number, number, number, number];
}

/** En el orden en que se enseñan: completa, mitades, tercios, dos tercios, cuartos. */
export const ZONES: ZoneInfo[] = [
  { id: "maximized", label: "Toda la pantalla", sixths: [0, 0, 6, 6] },
  { id: "leftHalf", label: "Mitad izquierda", sixths: [0, 0, 3, 6] },
  { id: "rightHalf", label: "Mitad derecha", sixths: [3, 0, 3, 6] },
  { id: "topHalf", label: "Mitad de arriba", sixths: [0, 0, 6, 3] },
  { id: "bottomHalf", label: "Mitad de abajo", sixths: [0, 3, 6, 3] },
  { id: "leftThird", label: "Tercio izquierdo", sixths: [0, 0, 2, 6] },
  { id: "centerThird", label: "Tercio central", sixths: [2, 0, 2, 6] },
  { id: "rightThird", label: "Tercio derecho", sixths: [4, 0, 2, 6] },
  {
    id: "leftTwoThirds",
    label: "Dos tercios izquierdos",
    sixths: [0, 0, 4, 6],
  },
  { id: "rightTwoThirds", label: "Dos tercios derechos", sixths: [2, 0, 4, 6] },
  {
    id: "topLeft",
    label: "Cuarto de arriba a la izquierda",
    sixths: [0, 0, 3, 3],
  },
  {
    id: "topRight",
    label: "Cuarto de arriba a la derecha",
    sixths: [3, 0, 3, 3],
  },
  {
    id: "bottomLeft",
    label: "Cuarto de abajo a la izquierda",
    sixths: [0, 3, 3, 3],
  },
  {
    id: "bottomRight",
    label: "Cuarto de abajo a la derecha",
    sixths: [3, 3, 3, 3],
  },
];

export interface MonitorOption {
  /** Valor del desplegable: «primary», «underMouse» o «specific:<índice>». */
  value: string;
  label: string;
  choice: MonitorChoice;
}

/** Dónde está cada pantalla, de izquierda a derecha («izquierda», «centro», «derecha»). */
function positions(count: number): string[] {
  if (count === 2) return ["izquierda", "derecha"];
  if (count === 3) return ["izquierda", "centro", "derecha"];
  return Array.from(
    { length: count },
    (_, i) => `${i + 1}.ª desde la izquierda`,
  );
}

/**
 * Las opciones de «en qué pantalla»: la principal, la del ratón y, si hay varias, cada
 * una con dónde está («Pantalla 1, la de la derecha»). Si la guardada ya no está
 * conectada, se añade para no perderla al editar.
 */
export function monitorOptions(
  monitors: MonitorInfo[],
  current: MonitorChoice,
): MonitorOption[] {
  const options: MonitorOption[] = [
    {
      value: "primary",
      label: "la pantalla principal",
      choice: { kind: "primary" },
    },
    {
      value: "underMouse",
      label: "la pantalla donde esté el ratón",
      choice: { kind: "underMouse" },
    },
  ];
  if (monitors.length > 1) {
    const sorted = [...monitors].sort(
      (a, b) => a.bounds.left - b.bounds.left || a.bounds.top - b.bounds.top,
    );
    const where = positions(sorted.length);
    sorted.forEach((m, i) =>
      options.push({
        value: `specific:${i}`,
        label:
          `${m.name}, la de la ${where[i]}${m.primary ? " (principal)" : ""}`
            .replace("la de la centro", "la del centro")
            .replace(/la de la (\d)/, "la $1"),
        choice: {
          kind: "specific",
          name: m.name,
          left: m.bounds.left,
          top: m.bounds.top,
        },
      }),
    );
  }
  if (
    current.kind === "specific" &&
    !options.some((o) => sameChoice(o.choice, current))
  ) {
    options.push({
      value: "specific:saved",
      label: `${current.name} (no conectada: se usará la principal)`,
      choice: current,
    });
  }
  return options;
}

export function sameChoice(a: MonitorChoice, b: MonitorChoice): boolean {
  if (a.kind !== b.kind) return false;
  if (a.kind === "specific" && b.kind === "specific")
    return a.name === b.name && a.left === b.left && a.top === b.top;
  return true;
}
