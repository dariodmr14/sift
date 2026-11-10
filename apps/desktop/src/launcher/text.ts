/**
 * Pasa a minúsculas y quita tildes ("Normalización" → "normalizacion"), para que buscar
 * con o sin tilde dé lo mismo. `NFD` separa la letra de su tilde y la regex borra la tilde.
 */
export function normalize(text: string): string {
  return text
    .normalize("NFD")
    .replace(/\p{Diacritic}/gu, "")
    .toLowerCase();
}

/** Palabras de un texto ya normalizado. */
export function words(text: string): string[] {
  return normalize(text).split(/\s+/).filter(Boolean);
}
