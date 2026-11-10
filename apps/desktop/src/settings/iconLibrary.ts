// Librería de iconos (Lucide): búsqueda y generación del SVG que se guarda en el Espacio.

import { normalize, words } from "../launcher/text";

/** Formato de Lucide: lista de elementos SVG `[etiqueta, atributos]`. */
export type IconNode = [string, Record<string, string | number>][];

const ESCAPES: Record<string, string> = {
  "&": "&amp;",
  "<": "&lt;",
  ">": "&gt;",
  '"': "&quot;",
};
const escapeAttr = (v: string) => v.replace(/[&<>"]/g, (c) => ESCAPES[c]);
const isSafeName = (n: string) => /^[a-zA-Z][a-zA-Z0-9-]*$/.test(n);

/**
 * SVG de un icono, con trazo `currentColor` para que tome el color del texto. Los
 * nombres de etiqueta y atributo se filtran y los valores se escapan: el SVG se guarda en
 * la base de datos y no debe poder llevar nada inesperado.
 */
export function buildSvg(node: IconNode): string {
  const children = node
    .filter(([tag]) => isSafeName(tag))
    .map(([tag, attrs]) => {
      const a = Object.entries(attrs)
        .filter(([k]) => isSafeName(k) && !k.toLowerCase().startsWith("on"))
        .map(([k, v]) => `${k}="${escapeAttr(String(v))}"`)
        .join(" ");
      return `<${tag} ${a}/>`;
    })
    .join("");
  return (
    '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" ' +
    'stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">' +
    `${children}</svg>`
  );
}

/** "ArrowDownCircle" → "arrow down circle". */
export function humanize(name: string): string {
  return name
    .replace(/([a-z0-9])([A-Z])/g, "$1 $2")
    .replace(/([A-Z])([A-Z][a-z])/g, "$1 $2")
    .toLowerCase();
}

/**
 * Palabras en español → nombres de Lucide (en inglés). No pretende ser un diccionario:
 * solo lo habitual al montar Espacios de estudio, trabajo u ocio.
 */
const SPANISH: Record<string, string[]> = {
  carpeta: ["folder"],
  archivo: ["file"],
  documento: ["file text"],
  base: ["database"],
  datos: ["database"],
  libro: ["book"],
  apuntes: ["notebook", "book open"],
  codigo: ["code"],
  programacion: ["code"],
  terminal: ["terminal"],
  web: ["globe"],
  internet: ["globe"],
  musica: ["music"],
  juego: ["gamepad"],
  juegos: ["gamepad"],
  correo: ["mail"],
  calendario: ["calendar"],
  casa: ["house"],
  trabajo: ["briefcase"],
  universidad: ["graduation cap"],
  clase: ["graduation cap", "school"],
  estudiar: ["book open", "graduation cap"],
  video: ["video"],
  foto: ["image", "camera"],
  dinero: ["wallet", "banknote"],
  red: ["network"],
  redes: ["network"],
  servidor: ["server"],
  nube: ["cloud"],
  chat: ["message circle"],
  ajustes: ["settings"],
  estrella: ["star"],
  corazon: ["heart"],
  cohete: ["rocket"],
  diseno: ["palette", "pen tool"],
  matematicas: ["calculator", "sigma"],
  ciencia: ["flask conical", "atom"],
};

/** Palabras que no aportan al buscar ("base *de* datos"). */
const STOPWORDS = new Set([
  "de",
  "del",
  "la",
  "el",
  "los",
  "las",
  "y",
  "para",
  "con",
  "en",
]);

export interface IconHit {
  name: string;
  label: string;
}

/**
 * Busca iconos por nombre (en inglés o con el diccionario en español). Se queda con los
 * que encajan con todas las palabras; primero los que empiezan por la búsqueda.
 */
export function searchIcons(
  query: string,
  names: string[],
  limit = 120,
): IconHit[] {
  const q = words(query).filter((w) => !STOPWORDS.has(w));
  const all = names.map((name) => ({ name, label: humanize(name) }));
  if (q.length === 0) return all.slice(0, limit);
  // Cada palabra de la búsqueda puede ser ella misma o su traducción.
  const options = q.map((w) => [w, ...(SPANISH[w] ?? [])]);
  const matches = all.filter(({ label }) =>
    options.every((alts) => alts.some((alt) => label.includes(normalize(alt)))),
  );
  const first = q[0];
  return matches
    .sort(
      (a, b) =>
        Number(b.label.startsWith(first)) - Number(a.label.startsWith(first)),
    )
    .slice(0, limit);
}
