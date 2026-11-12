// Cuándo tiene sentido sugerir rutas del índice (lógica pura, para probarla).

/**
 * Se sugiere mientras escribes un nombre («tema4», «apuntes bd»), no si ya es una ruta
 * completa («C:\Users\…» o «\\servidor\…»): entonces sabes dónde está y las sugerencias
 * por nombre solo estorbarían. Con menos de 2 letras tampoco (saldría casi todo).
 */
export function shouldSuggest(text: string): boolean {
  const t = text.trim();
  if (/^[a-z]:/i.test(t) || t.startsWith("\\\\") || t.startsWith("/")) {
    return false;
  }
  return t.replace(/[^\p{L}\p{N}]/gu, "").length >= 2;
}
