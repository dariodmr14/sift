// Textos de Ajustes → Carpetas en lenguaje de persona (sin React, para probarlos).
import type { IndexStatus } from "../bindings";

/** 1240 → «1.240». (`toLocaleString("es")` no pone el punto con 4 cifras: aquí sí.) */
export function count(n: number): string {
  return String(Math.round(n)).replace(/\B(?=(\d{3})+(?!\d))/g, ".");
}

/** «1 archivo», «1.240 archivos». */
export function files(n: number): string {
  return `${count(n)} ${n === 1 ? "archivo" : "archivos"}`;
}

/** «hace 3 min», «hace 2 h»… a partir de una fecha ISO de SQLite. */
export function since(
  iso: string | null | undefined,
  now = Date.now(),
): string {
  if (!iso) return "aún no";
  const secs = Math.max(0, Math.round((now - Date.parse(iso)) / 1000));
  if (secs < 60) return "hace un momento";
  if (secs < 3600) return `hace ${Math.round(secs / 60)} min`;
  if (secs < 86400) return `hace ${Math.round(secs / 3600)} h`;
  return `hace ${Math.round(secs / 86400)} d`;
}

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
