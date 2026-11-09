// Resultados de prueba para la Fase 1. En la Fase 2 los sustituye la búsqueda real.

export type ResultKind =
  "pdf" | "doc" | "note" | "folder" | "app" | "web" | "space";

export interface SearchResult {
  id: string;
  title: string;
  /** Ruta, URL o descripción corta que se muestra debajo del título. */
  subtitle: string;
  kind: ResultKind;
}

export const MOCK_RESULTS: SearchResult[] = [
  {
    id: "1",
    title: "Tema 4 - Normalización.pdf",
    subtitle: "Documentos\\Uni\\Bases de Datos\\Teoría",
    kind: "pdf",
  },
  {
    id: "2",
    title: "Apuntes 3FN y BCNF.md",
    subtitle: "Documentos\\Uni\\Bases de Datos\\Apuntes",
    kind: "note",
  },
  {
    id: "3",
    title: "Práctica 2 - Diseño relacional.docx",
    subtitle: "Documentos\\Uni\\Bases de Datos\\Prácticas",
    kind: "doc",
  },
  {
    id: "4",
    title: "Bases de datos",
    subtitle: "Espacio · carpeta, apuntes, Docker y Campus Virtual",
    kind: "space",
  },
  {
    id: "5",
    title: "Bases de Datos",
    subtitle: "Documentos\\Uni\\Bases de Datos",
    kind: "folder",
  },
  {
    id: "6",
    title: "DBeaver",
    subtitle: "Aplicación",
    kind: "app",
  },
  {
    id: "7",
    title: "Campus Virtual",
    subtitle: "https://www.campusvirtual.uniovi.es",
    kind: "web",
  },
  {
    id: "8",
    title: "Tema 5 - Transacciones.pdf",
    subtitle: "Documentos\\Uni\\Bases de Datos\\Teoría",
    kind: "pdf",
  },
];

/**
 * Pasa a minúsculas y quita tildes ("Normalización" → "normalizacion"), para que buscar
 * con o sin tilde dé lo mismo. `NFD` separa la letra de su tilde y la regex borra la tilde.
 */
export function normalize(text: string): string {
  return text
    .normalize("NFD")
    .replace(/\p{Diacritic}/gu, "")
    .toLowerCase();
}

/**
 * Un resultado encaja si todas las palabras de la búsqueda aparecen en su título o
 * subtítulo, en cualquier orden. Con la búsqueda vacía se muestran todos.
 */
export function filterResults(
  query: string,
  items: SearchResult[],
): SearchResult[] {
  const words = normalize(query).split(/\s+/).filter(Boolean);
  if (words.length === 0) return items;
  return items.filter((item) => {
    const haystack = normalize(`${item.title} ${item.subtitle}`);
    return words.every((w) => haystack.includes(w));
  });
}
