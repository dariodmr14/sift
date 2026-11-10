import { describe, expect, it } from "vitest";
import { dropSlot, isNoop, reorder, targetIndex } from "./reorder";

// Tres filas de 40 px una debajo de otra: 0–40, 40–80, 80–120.
const boxes = [0, 40, 80].map((top) => ({ top, height: 40 }));

describe("dropSlot", () => {
  it("por encima de la mitad de una fila, el hueco es antes de ella", () => {
    expect(dropSlot(5, boxes)).toBe(0);
    expect(dropSlot(19, boxes)).toBe(0);
    expect(dropSlot(21, boxes)).toBe(1);
    expect(dropSlot(61, boxes)).toBe(2);
  });

  it("por debajo de la última mitad, el hueco del final", () => {
    expect(dropSlot(101, boxes)).toBe(3);
    expect(dropSlot(500, boxes)).toBe(3);
  });

  it("por encima de todo, el primer hueco", () => {
    expect(dropSlot(-50, boxes)).toBe(0);
  });

  it("lista vacía: hueco 0", () => {
    expect(dropSlot(10, [])).toBe(0);
  });
});

describe("reorder", () => {
  const list = ["a", "b", "c", "d"];

  it("baja un elemento", () => {
    expect(reorder(list, 0, 2)).toEqual(["b", "a", "c", "d"]);
    expect(reorder(list, 0, 4)).toEqual(["b", "c", "d", "a"]);
  });

  it("sube un elemento", () => {
    expect(reorder(list, 3, 0)).toEqual(["d", "a", "b", "c"]);
    expect(reorder(list, 2, 1)).toEqual(["a", "c", "b", "d"]);
  });

  it("soltarlo junto a sí mismo no cambia nada (misma lista)", () => {
    expect(reorder(list, 1, 1)).toBe(list);
    expect(reorder(list, 1, 2)).toBe(list);
    expect(isNoop(1, 1) && isNoop(1, 2)).toBe(true);
    expect(isNoop(1, 3)).toBe(false);
  });

  it("índices fuera de rango no rompen nada", () => {
    expect(reorder(list, 9, 0)).toBe(list);
  });

  it("targetIndex compensa el hueco que deja el elemento al salir", () => {
    expect(targetIndex(0, 3)).toBe(2);
    expect(targetIndex(3, 1)).toBe(1);
  });
});
