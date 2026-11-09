import { describe, expect, it } from "vitest";
import { moveSelection } from "./selection";

describe("moveSelection", () => {
  it("avanza y retrocede", () => {
    expect(moveSelection(0, 1, 5)).toBe(1);
    expect(moveSelection(3, -1, 5)).toBe(2);
  });

  it("da la vuelta en los extremos", () => {
    expect(moveSelection(4, 1, 5)).toBe(0);
    expect(moveSelection(0, -1, 5)).toBe(4);
  });

  it("sin selección, ↓ va al primero y ↑ al último", () => {
    expect(moveSelection(-1, 1, 5)).toBe(0);
    expect(moveSelection(-1, -1, 5)).toBe(4);
  });

  it("con la lista vacía no selecciona nada", () => {
    expect(moveSelection(0, 1, 0)).toBe(-1);
  });
});
