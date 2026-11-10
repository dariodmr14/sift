// Lógica pura de "arrastrar para reordenar" (sin React ni DOM), para poder probarla.
//
// Se trabaja con *huecos*: en una lista de n elementos hay n + 1 sitios donde soltar
// (antes del primero, entre cada dos, después del último). Soltar en el hueco i deja el
// elemento justo antes del que ahora está en la posición i.

/** Posición vertical de cada elemento en pantalla (de `getBoundingClientRect`). */
export interface Box {
  top: number;
  height: number;
}

/**
 * Hueco donde caería el elemento si se soltara con el puntero en `y`: el primer elemento
 * cuya mitad queda por debajo del puntero marca el hueco; si no hay, el final.
 */
export function dropSlot(y: number, boxes: Box[]): number {
  const i = boxes.findIndex((b) => y < b.top + b.height / 2);
  return i === -1 ? boxes.length : i;
}

/**
 * ¿Cambia algo soltando `from` en el hueco `slot`? Los dos huecos que tocan al propio
 * elemento (justo antes y justo después) lo dejan donde estaba.
 */
export function isNoop(from: number, slot: number): boolean {
  return slot === from || slot === from + 1;
}

/**
 * Posición final del elemento tras soltarlo en `slot`. Al sacarlo de la lista, los
 * huecos de después de él se desplazan uno hacia arriba.
 */
export function targetIndex(from: number, slot: number): number {
  return slot > from ? slot - 1 : slot;
}

/** Mueve el elemento `from` al hueco `slot`. Si no cambia nada, devuelve la misma lista. */
export function reorder<T>(list: T[], from: number, slot: number): T[] {
  if (from < 0 || from >= list.length || isNoop(from, slot)) return list;
  const copy = [...list];
  const [item] = copy.splice(from, 1);
  copy.splice(targetIndex(from, slot), 0, item);
  return copy;
}
