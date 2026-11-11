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
 * «Al día · 1.240 archivos», «Buscando cambios…», «Leyendo 32 de 120 archivos», «En
 * pausa…». `totalFiles` es la suma de archivos de las carpetas (de `listRoots`).
 */
export function statusLine(
  status: IndexStatus | null,
  totalFiles: number,
  folders: number,
): StatusLine {
  if (folders === 0)
    return { text: "Añade una carpeta para empezar.", progress: null };
  if (!status) return { text: "Comprobando…", progress: null };
  const reading = status.running && status.total > 0;
  if (reading && status.phase === "reading") {
    return {
      text: `Leyendo ${count(Math.min(status.done + 1, status.total))} de ${files(status.total)}`,
      progress: status.done / status.total,
    };
  }
  if (reading && status.phase === "paused") {
    return {
      text: `En pausa mientras usas el PC · ${count(status.done)} de ${files(status.total)}`,
      progress: status.done / status.total,
    };
  }
  if (status.running && status.phase === "updating")
    return { text: "Actualizando lo que acabas de cambiar…", progress: null };
  if (status.running)
    return { text: "Buscando cambios…", progress: "indeterminate" };
  return { text: `Al día · ${files(totalFiles)}`, progress: null };
}

/**
 * Avisos que no son errores pero conviene saber: sin vigilancia en tiempo real, los
 * cambios se ven al volver a abrir Sift; sin lector de PDF, los PDF van solo por nombre.
 */
export function statusNotes(status: IndexStatus | null): string[] {
  if (!status) return [];
  const notes: string[] = [];
  if (!status.watching)
    notes.push(
      "Los cambios en tus carpetas se verán la próxima vez que abras Sift: no se pudieron vigilar en tiempo real.",
    );
  if (!status.readsPdf)
    notes.push(
      "Los PDF se encuentran solo por su nombre: falta el lector de PDF (pdfium.dll).",
    );
  return notes;
}

/** Por qué no se pudo leer un archivo, dicho para cualquiera (códigos de Rust). */
export const UNREAD_REASON: Record<string, { label: string; hint: string }> = {
  unsupported: {
    label: "Formato no compatible",
    hint: "Por dentro no son texto, aunque la extensión lo parezca.",
  },
  tooLarge: {
    label: "Demasiado grandes",
    hint: "Ocupan más de 5 MB. Se encuentran por su nombre.",
  },
  noPermission: {
    label: "Sin permiso",
    hint: "Windows no deja abrirlos con tu usuario.",
  },
  protected: {
    label: "Protegidos con contraseña",
    hint: "Se encuentran por su nombre.",
  },
  damaged: {
    label: "Dañados",
    hint: "Están incompletos o no se pueden abrir.",
  },
  other: {
    label: "Otros problemas",
    hint: "No se pudieron abrir al leerlos.",
  },
};

/** Texto de un motivo; uno desconocido cuenta como «otros». */
export function unreadReason(kind: string): { label: string; hint: string } {
  return UNREAD_REASON[kind] ?? UNREAD_REASON.other;
}
