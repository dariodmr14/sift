// Lo que la vista previa de la barra dice del resultado: por qué está ahí, y cuándo la
// abre la flecha derecha. Todo puro, sin React.

import type { FileHit } from "../bindings";
import { normalize } from "./text";

export type PreviewReason =
  | { kind: "contains"; words: string[] }
  | { kind: "meaning" }
  | { kind: "name" };

const MAX_WORDS = 3;

/**
 * Por qué sale el resultado. Por significado manda (no tiene tus palabras dentro); si
 * hay coincidencias en el texto, cuáles (únicas sin distinguir mayúsculas ni tildes,
 * la primera forma vista, hasta 3); si no, solo el nombre.
 */
export function previewReason(
  hit: Pick<FileHit, "snippet" | "byMeaning">,
): PreviewReason {
  if (hit.byMeaning) return { kind: "meaning" };
  const seen = new Set<string>();
  const found: string[] = [];
  for (const s of hit.snippet) {
    if (!s.hit) continue;
    const key = normalize(s.text);
    if (seen.has(key)) continue;
    seen.add(key);
    found.push(s.text);
  }
  if (found.length === 0) return { kind: "name" };
  return { kind: "contains", words: found.slice(0, MAX_WORDS) };
}

export function reasonLabel(r: PreviewReason): string {
  switch (r.kind) {
    case "contains":
      return `Contiene ${r.words.map((w) => `«${w}»`).join(", ")}`;
    case "meaning":
      return "Trata de lo mismo";
    case "name":
      return "Por el nombre";
  }
}

/** ¿Abre la vista previa → con este cursor? Solo con el cursor al final y sin selección. */
export function caretAtEnd(
  value: string,
  start: number | null,
  end: number | null,
): boolean {
  return start === value.length && end === value.length;
}
