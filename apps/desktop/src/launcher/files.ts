// Ayudantes puros de la búsqueda unificada (sin React, para poder probarlos).

/** Secciones de la lista de resultados, en este orden. */
export type Section = "spaces" | "files" | "commands";

export const SECTION_LABEL: Record<Section, string> = {
  spaces: "Espacios",
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
 * Índices en los que empieza una sección nueva (para pintar su cabecera). Los elementos
 * llegan ya ordenados por sección.
 */
export function sectionStarts(sections: Section[]): Set<number> {
  const starts = new Set<number>();
  sections.forEach((s, i) => {
    if (i === 0 || sections[i - 1] !== s) starts.add(i);
  });
  return starts;
}

/** ¿Es esta respuesta la última que se pidió? (las que llegan tarde se descartan) */
export function isLatest(responseSeq: number, latestSeq: number): boolean {
  return responseSeq === latestSeq;
}
