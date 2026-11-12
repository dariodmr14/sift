// Filtros rápidos de la barra: «pdf contrato» busca solo en PDF (lógica pura).
//
// Empiezas con una palabra clave y un espacio: la palabra se convierte en una etiqueta
// junto a la caja y el resto es la búsqueda. Retroceso al principio la quita. Las
// extensiones de cada filtro las pone Rust (`sift_extract::type_filter`): aquí solo van
// el id, el nombre y las palabras que lo activan.
//
// Solo palabras que casi nadie busca por su contenido: «documento», «presentación» o
// «notas» se quedaron fuera porque son también cosas que se buscan («notas del examen»).

import { normalize } from "./text";

export interface TypeFilter {
  id: string;
  /** Lo que dice la etiqueta. */
  label: string;
  /** En plural, para «Buscar en tus PDF…». */
  plural: string;
  /** Palabras que lo activan (sin tildes y en minúsculas). */
  keywords: string[];
}

export const TYPE_FILTERS: TypeFilter[] = [
  { id: "pdf", label: "PDF", plural: "PDF", keywords: ["pdf"] },
  {
    id: "word",
    label: "Word",
    plural: "documentos de Word",
    keywords: ["word"],
  },
  {
    id: "slides",
    label: "Presentaciones",
    plural: "presentaciones",
    keywords: ["powerpoint", "diapositivas"],
  },
  {
    id: "sheet",
    label: "Excel",
    plural: "hojas de cálculo",
    keywords: ["excel"],
  },
  {
    id: "code",
    label: "Código",
    plural: "archivos de código",
    keywords: ["codigo"],
  },
  {
    id: "image",
    label: "Imágenes",
    plural: "imágenes",
    keywords: ["imagen", "imagenes", "foto", "fotos"],
  },
  {
    id: "video",
    label: "Vídeos",
    plural: "vídeos",
    keywords: ["video", "videos"],
  },
];

/**
 * Si `text` empieza por una palabra clave seguida de un espacio, el filtro y lo que
 * queda detrás. Sin el espacio no se activa: mientras escribes «pdfs» o «codigos» no
 * debe saltar nada.
 */
export function detectFilter(
  text: string,
): { filter: TypeFilter; rest: string } | null {
  const m = /^\s*(\S+)\s(.*)$/s.exec(text);
  if (!m) return null;
  const word = normalize(m[1]);
  const filter = TYPE_FILTERS.find((f) => f.keywords.includes(word));
  return filter ? { filter, rest: m[2] } : null;
}
