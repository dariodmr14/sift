// Ayudantes puros de la búsqueda unificada (sin React, para poder probarlos).

/** Secciones de la lista de resultados, en este orden. */
export type Section = "spaces" | "recent" | "files" | "commands";

export const SECTION_LABEL: Record<Section, string> = {
  spaces: "Espacios",
  recent: "Abierto recientemente",
  files: "Archivos",
  commands: "Comandos",
};

/**
 * Carpeta de un archivo, acortada para que quepa en una línea: se quedan las últimas
 * `keep` carpetas («…\Uni\Bases de Datos\Teoría»). Acepta `\` y `/`.
 */
export function shortFolder(path: string, keep = 3): string {
  const parts = path.split(/[\\/]+/).filter(Boolean);
  parts.pop(); // el nombre del archivo
  if (parts.length <= keep) return parts.join("\\");
  return `…\\${parts.slice(-keep).join("\\")}`;
}

/**
 * Índices en los que empieza una sección nueva (para pintar su cabecera). Si alguna
 * sección sale en dos trozos (Espacios intercalados entre archivos), ninguna cabecera:
 * «Archivos · Espacios · Archivos» confundiría más que ayudaría. Entonces cada Espacio
 * lo dice en su subtítulo.
 */
export function sectionStarts(sections: Section[]): Set<number> {
  const starts = new Set<number>();
  const seen = new Set<Section>();
  for (const [i, s] of sections.entries()) {
    if (i > 0 && sections[i - 1] === s) continue;
    if (seen.has(s)) return new Set();
    seen.add(s);
    starts.add(i);
  }
  return starts;
}

/** ¿Es esta respuesta la última que se pidió? (las que llegan tarde se descartan) */
export function isLatest(responseSeq: number, latestSeq: number): boolean {
  return responseSeq === latestSeq;
}
