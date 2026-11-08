/**
 * Mueve la selección `delta` posiciones dentro de una lista de `count` elementos,
 * dando la vuelta en los extremos (↓ en el último vuelve al primero).
 * Con la lista vacía devuelve -1 (nada seleccionado).
 */
export function moveSelection(
  index: number,
  delta: number,
  count: number,
): number {
  if (count === 0) return -1;
  if (index < 0) return delta > 0 ? 0 : count - 1;
  // El `+ count` evita módulos negativos: en JS, -1 % 5 === -1.
  return (((index + delta) % count) + count) % count;
}
