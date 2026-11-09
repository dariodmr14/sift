// Comandos integrados de Sift: resultados que no son archivos ni Espacios, sino acciones
// de la propia app ("Ajustes", y más adelante "Crear Espacio", "Exportar"…).
// Se encuentran escribiendo, igual que todo lo demás en un lanzador.

import { normalize } from "./results";

export type BuiltinId = "settings";

export interface BuiltinCommand {
  id: BuiltinId;
  title: string;
  subtitle: string;
  /** Palabras que lo encuentran, además del título. */
  keywords: string[];
  /** Atajo de teclado directo, para mostrarlo junto al resultado. */
  shortcut?: string;
}

export const BUILTINS: BuiltinCommand[] = [
  {
    id: "settings",
    title: "Ajustes de Sift",
    subtitle: "Atajo, Espacios, iconos y arranque",
    keywords: [
      "ajustes",
      "configuracion",
      "preferencias",
      "opciones",
      "settings",
    ],
    shortcut: "Ctrl+,",
  },
];

/** Longitud mínima de la búsqueda para mostrar comandos: con una letra habría ruido. */
const MIN_QUERY = 2;

/**
 * Un comando encaja si cada palabra de la búsqueda es el *principio* de alguna palabra
 * del título o de sus palabras clave ("aju" → "ajustes", "conf" → "configuración").
 * Prefijo y no "contiene", para que "tes" no saque "ajustes".
 */
export function matchBuiltins(
  query: string,
  commands = BUILTINS,
): BuiltinCommand[] {
  const words = normalize(query).split(/\s+/).filter(Boolean);
  if (words.join("").length < MIN_QUERY) return [];
  return commands.filter((cmd) => {
    const vocabulary = normalize([cmd.title, ...cmd.keywords].join(" ")).split(
      /\s+/,
    );
    return words.every((w) => vocabulary.some((v) => v.startsWith(w)));
  });
}
