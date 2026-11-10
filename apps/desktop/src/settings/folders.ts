// Textos de Ajustes → Carpetas en lenguaje de persona (sin React, para probarlos).
import type { IndexStatus } from "../bindings";
import { count, files } from "../format";

const parts = (path: string) => path.split(/[\\/]+/).filter(Boolean);

/** Nombre de la carpeta: «C:\Users\Ana\Documents\Uni» → «Uni». */
export function folderName(path: string): string {
  const p = parts(path);
  return p[p.length - 1] ?? path;
}

/**
 * Dónde está, abreviado: la carpeta que la contiene, con los tramos del medio cambiados
 * por «…». «C:\Users\Ana\Documents\GitHub\Sift» → «C:\…\Documents\GitHub».
 */
export function shortParent(path: string): string {
  const p = parts(path).slice(0, -1);
  if (p.length <= 3) return p.join("\\") || path;
  return [p[0], "…", ...p.slice(-2)].join("\\");
}

/** Lo que dice la línea de estado, y cuánto llena la barra (null = sin barra). */
export interface StatusLine {
  text: string;
  /** 0–1 para una barra con total; "indeterminate" si aún no se sabe el total. */
  progress: number | "indeterminate" | null;
}

/**
 * «Al día · 1.240 archivos», «Buscando cambios…», «Leyendo 32 de 120 archivos».
 * `totalFiles` es la suma de archivos de las carpetas (de `listRoots`).
 */
export function statusLine(
  status: IndexStatus | null,
  totalFiles: number,
  folders: number,
): StatusLine {
  if (folders === 0)
    return { text: "Añade una carpeta para empezar.", progress: null };
  if (!status) return { text: "Comprobando…", progress: null };
  if (status.running && status.phase === "reading" && status.total > 0) {
    return {
      text: `Leyendo ${count(Math.min(status.done + 1, status.total))} de ${files(status.total)}`,
      progress: status.done / status.total,
    };
  }
  if (status.running)
    return { text: "Buscando cambios…", progress: "indeterminate" };
  return { text: `Al día · ${files(totalFiles)}`, progress: null };
}
