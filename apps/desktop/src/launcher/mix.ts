// Mezcla Espacios y archivos en una sola lista (Fase 6). Ver ADR 0011.
//
// - Si lo escrito es **exactamente** el nombre o un alias de un Espacio, ese Espacio va
//   primero, lo uses o no. Entre varios así, decide la frecencia.
// - Si la coincidencia es **parcial**, el Espacio entra entre los archivos con una
//   puntuación en su misma escala: la relevancia del mejor archivo encontrado (sin
//   frecencia) × lo bien que encaja × su frecencia. Así la frecencia decide entre un
//   Espacio y un archivo igual de buenos.
// - Los archivos no cambian de orden entre sí: los Espacios se intercalan.

import { score, type Searchable } from "./spaces";

/** Cuánto vale cada tipo de coincidencia parcial frente al mejor archivo. */
export const PARTIAL_WEIGHT: Record<1 | 2 | 3, number> = {
  /** El nombre o un alias empieza por lo escrito («bases d…»): como el mejor archivo. */
  3: 1,
  /** Cada palabra empieza alguna palabra del nombre («dat bas»). */
  2: 0.8,
  /** Cada palabra aparece en algún sitio («atos»). */
  1: 0.5,
};

export interface SpaceCandidate extends Searchable {
  /** Frecencia: por cuánto se multiplica (1 = nunca abierto). */
  boost: number;
}

// Los `f64` de Rust llegan como `number | null` (un NaN viaja como `null`): `null` se
// trata como neutro (puntuación 0, sin frecencia).
export interface FileCandidate {
  /** Puntuación final del archivo (ya con su frecencia). */
  score: number | null;
  /** Lo que su frecencia multiplica: `score / boost` es su relevancia sin frecencia. */
  boost: number | null;
}

export type Mixed<S, F> =
  { kind: "space"; item: S; exact: boolean } | { kind: "file"; item: F };

/**
 * La lista de la barra: primero los Espacios que encajan exactamente y después,
 * intercalados, los parciales y los archivos. Con la búsqueda vacía, todos los Espacios
 * en el orden de Ajustes.
 */
export function mixResults<S extends SpaceCandidate, F extends FileCandidate>(
  query: string,
  spaces: S[],
  files: F[],
): Mixed<S, F>[] {
  if (!query.trim()) {
    return spaces.map((item) => ({ kind: "space", item, exact: false }));
  }
  const matched = spaces
    .map((item, order) => ({ item, order, quality: score(query, item) }))
    .filter((s) => s.quality > 0);

  // A igual puntuación, el orden de Ajustes.
  const byScore = <T extends { order: number; value: number }>(a: T, b: T) =>
    b.value - a.value || a.order - b.order;

  const exact = matched
    .filter((s) => s.quality === 4)
    .map((s) => ({ ...s, value: s.item.boost }))
    .sort(byScore);

  // La referencia: la relevancia del mejor archivo, sin su frecencia (si no hay
  // archivos, 1: entonces solo importa el orden entre Espacios).
  const anchor = files.length
    ? Math.max(...files.map((f) => (f.score ?? 0) / (f.boost || 1)))
    : 1;
  const partial = matched
    .filter((s) => s.quality < 4)
    .map((s) => ({
      ...s,
      value: anchor * PARTIAL_WEIGHT[s.quality as 1 | 2 | 3] * s.item.boost,
    }))
    .sort(byScore);

  const out: Mixed<S, F>[] = exact.map((s) => ({
    kind: "space",
    item: s.item,
    exact: true,
  }));
  // Intercalar sin reordenar los archivos: un Espacio entra delante del primer archivo
  // que puntúa menos que él (a igualdad, el Espacio primero).
  let f = 0;
  for (const s of partial) {
    while (f < files.length && (files[f].score ?? 0) > s.value) {
      out.push({ kind: "file", item: files[f++] });
    }
    out.push({ kind: "space", item: s.item, exact: false });
  }
  while (f < files.length) out.push({ kind: "file", item: files[f++] });
  return out;
}
