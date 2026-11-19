// La introducción guiada de la primera vez (FEEDBACK 2026-11-18, punto 6, y
// `docs/proceso/propuesta-introduccion.md`): cinco pasos, uno por pantalla. Lógica pura, para
// probarla sin React.

import type { Draft } from "./draft";

export const INTRO_STEPS = [
  { id: "hotkey", title: "Tu atajo" },
  { id: "folders", title: "Dónde buscar" },
  { id: "search", title: "Prueba a buscar" },
  { id: "smart", title: "Búsqueda inteligente" },
  { id: "space", title: "Tu primer Espacio" },
] as const;

export type IntroStepId = (typeof INTRO_STEPS)[number]["id"];

/** «Paso 2 de 5: Dónde buscar», para el lector de pantalla y el título de la ventana. */
export function stepLabel(index: number): string {
  return `Paso ${index + 1} de ${INTRO_STEPS.length}: ${INTRO_STEPS[index].title}`;
}

/** Hacia dónde entra el paso nuevo: desde la derecha al avanzar, desde la izquierda al volver. */
export type Direction = "forward" | "back";

export function clampStep(index: number): number {
  return Math.max(0, Math.min(INTRO_STEPS.length - 1, index));
}

/** «Alt+Space» → «Alt+Espacio», como en el teclado español. */
export function hotkeyText(hotkey: string): string {
  return hotkey.replace(/\bSpace\b/, "Espacio");
}

/**
 * El primer Espacio que se crea en la introducción: un nombre y, como acciones, abrir las
 * carpetas marcadas y una web si se escribió. Sin nombre o sin acciones no hay Espacio.
 */
export function firstSpaceDraft(
  name: string,
  folders: string[],
  url: string,
): Draft | null {
  const trimmed = name.trim();
  const web = url.trim();
  const actions: Draft["actions"] = [
    ...folders.map((path) => ({ kind: "openFolder" as const, path })),
    ...(web
      ? [
          {
            kind: "openUrl" as const,
            url: /^[a-z][a-z0-9+.-]*:/i.test(web) ? web : `https://${web}`,
          },
        ]
      : []),
  ];
  if (!trimmed || actions.length === 0) return null;
  return { name: trimmed, aliases: [], icon: { kind: "auto" }, actions };
}
