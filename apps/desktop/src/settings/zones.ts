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

/** Cómo se enseñan: una fila por grupo, con su nombre. `null` es «No colocar». */
export const ZONE_GROUPS: { label: string; zones: (Zone | null)[] }[] = [
  { label: "General", zones: [null, "maximized"] },
  {
    label: "Mitades",
    zones: ["leftHalf", "rightHalf", "topHalf", "bottomHalf"],
  },
  {
    label: "Tercios",
    zones: [
      "leftThird",
      "centerThird",
      "rightThird",
      "leftTwoThirds",
      "rightTwoThirds",
    ],
  },
  {
    label: "Cuartos",
    zones: ["topLeft", "topRight", "bottomLeft", "bottomRight"],
  },
];

export interface MonitorOption {
  /** Valor del desplegable: «primary», «underMouse» o «specific:<índice>». */
  value: string;
  label: string;
  /** Lo corto, para el resumen: «Pantalla 2», «donde esté el ratón». */
  short: string;
  choice: MonitorChoice;
}

/** El nombre que pone Windows cuando no sabe el modelo («Pantalla 2»): no dice nada más. */
const GENERIC_NAME = /^Pantalla \d+$/;

/**
 * Las opciones de «Pantalla», como en Configuración de Windows: cada pantalla con el número
 * que le da Windows, su modelo y cuál es la principal («Pantalla 1 — DELL U2720Q
 * (principal)»), y al final «Donde esté el ratón». La principal se guarda como «la principal» y no como esa pantalla
 * concreta: si cambias de principal en Windows, la sigue. Si la guardada ya no está
 * conectada, se añade para no perderla al editar.
 */
export function monitorOptions(
  monitors: MonitorInfo[],
  current: MonitorChoice,
): MonitorOption[] {
  // El número de Windows manda; si no lo hay (0), de izquierda a derecha.
  const sorted = [...monitors].sort(
    (a, b) =>
      (a.number || 99) - (b.number || 99) ||
      a.bounds.left - b.bounds.left ||
      a.bounds.top - b.bounds.top,
  );
  const options: MonitorOption[] = sorted.map((m, i) => {
    const short = `Pantalla ${m.number || i + 1}`;
    const label = `${short}${GENERIC_NAME.test(m.name) ? "" : ` — ${m.name}`}`;
    // Un Espacio guardado antes puede tener la principal elegida como pantalla concreta:
    // se respeta tal cual.
    const savedAsThis =
      current.kind === "specific" &&
      current.name === m.name &&
      current.left === m.bounds.left &&
      current.top === m.bounds.top;
    return m.primary && !savedAsThis
      ? {
          value: "primary",
          label: `${label} (principal)`,
          short,
          choice: { kind: "primary" },
        }
      : {
          value: `specific:${i}`,
          label: m.primary ? `${label} (principal)` : label,
          short,
          choice: {
            kind: "specific",
            name: m.name,
            left: m.bounds.left,
            top: m.bounds.top,
          },
        };
  });
  // Mientras llega la lista de pantallas (o si Windows no da ninguna).
  if (options.length === 0)
    options.unshift({
      value: "primary",
      label: "Pantalla principal",
      short: "pantalla principal",
      choice: { kind: "primary" },
    });
  options.push({
    value: "underMouse",
    label: "Donde esté el ratón",
    short: "donde esté el ratón",
    choice: { kind: "underMouse" },
  });
  if (
    current.kind === "specific" &&
    !options.some((o) => sameChoice(o.choice, current))
  ) {
    options.push({
      value: "specific:saved",
      label: `${current.name} (no conectada: se usará la principal)`,
      short: current.name,
      choice: current,
    });
  }
  return options;
}

/**
 * Una línea con lo elegido, para cuando «Colocar su ventana» está plegado:
 * «Mitad derecha · Pantalla 2» o «No colocar», y si se usa la ya abierta.
 */
export function placementSummary(
  value: { zone: Zone | null; monitor: MonitorChoice; reuseWindow: boolean },
  options: MonitorOption[],
): string {
  const parts: string[] = [];
  if (value.zone) {
    parts.push(ZONES.find((z) => z.id === value.zone)?.label ?? value.zone);
    const monitor = options.find((o) => sameChoice(o.choice, value.monitor));
    if (monitor) parts.push(monitor.short);
  } else {
    parts.push("No colocar");
  }
  if (value.reuseWindow) parts.push("usar la ya abierta");
  return parts.join(" · ");
}

export function sameChoice(a: MonitorChoice, b: MonitorChoice): boolean {
  if (a.kind !== b.kind) return false;
  if (a.kind === "specific" && b.kind === "specific")
    return a.name === b.name && a.left === b.left && a.top === b.top;
  return true;
}
