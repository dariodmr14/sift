// Lo que se anuncia al lector de pantalla cuando cambian los resultados (F13).

import type { TypeFilter } from "./filters";

export function resultsAnnouncement(args: {
  query: string;
  filter: TypeFilter | null;
  count: number;
  empty: boolean;
}): string {
  const { query, filter, count, empty } = args;
  if (empty) return "";
  const where = filter ? ` en tus ${filter.plural}` : "";
  if (filter && !query.trim()) return `Buscando solo en tus ${filter.plural}`;
  if (count === 0) return `Sin resultados${where}`;
  if (count === 1) return `1 resultado${where}`;
  return `${count.toLocaleString("es-ES")} resultados${where}`;
}
