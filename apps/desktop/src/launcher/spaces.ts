// Buscar Espacios por nombre y alias desde la barra.

import { normalize, words } from "./text";

/** Lo mínimo que necesita la búsqueda de un Espacio. */
export interface Searchable {
  name: string;
  aliases: string[];
}

/**
 * Qué tal encaja `query` con un Espacio. 0 = no encaja; más alto = mejor.
 * - 4: es exactamente el nombre o un alias ("bd").
 * - 3: el nombre o un alias empieza por la búsqueda ("bases d…").
 * - 2: cada palabra de la búsqueda es el principio de alguna palabra ("dat bas").
 * - 1: cada palabra aparece en algún sitio ("atos").
 */
export function score(query: string, space: Searchable): number {
  const q = normalize(query).trim();
  if (!q) return 0;
  const names = [space.name, ...space.aliases].map((n) => normalize(n).trim());
  if (names.some((n) => n === q)) return 4;
  if (names.some((n) => n.startsWith(q))) return 3;
  const qWords = words(q);
  const vocabulary = names.flatMap((n) => words(n));
  if (qWords.every((w) => vocabulary.some((v) => v.startsWith(w)))) return 2;
  if (qWords.every((w) => names.some((n) => n.includes(w)))) return 1;
  return 0;
}

/**
 * Espacios que encajan con la búsqueda, los mejores primero. A igual puntuación se
 * respeta el orden que el usuario eligió en Ajustes. Con la búsqueda vacía, todos.
 */
export function matchSpaces<T extends Searchable>(
  query: string,
  spaces: T[],
): T[] {
  if (!query.trim()) return spaces;
  return spaces
    .map((space, order) => ({ space, order, s: score(query, space) }))
    .filter((x) => x.s > 0)
    .sort((a, b) => b.s - a.s || a.order - b.order)
    .map((x) => x.space);
}
