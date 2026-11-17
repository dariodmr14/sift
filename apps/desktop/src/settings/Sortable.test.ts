import { describe, expect, it } from "vitest";
import { composeTransition } from "./Sortable";

describe("transición del elemento en la mano", () => {
  it("une la de dnd-kit con la del fondo y la sombra", () => {
    const t = composeTransition("transform 180ms", false);
    expect(t).toMatch(/^transform 180ms, background-color /);
    expect(t).toContain("box-shadow");
  });
  it("sin la de dnd-kit (al soltar) deja la del fondo y la sombra", () => {
    const t = composeTransition(undefined, false);
    expect(t).toMatch(/^background-color /);
    expect(t).toContain("box-shadow");
  });
  it("con «reducir movimiento» no añade nada", () => {
    expect(composeTransition(undefined, true)).toBeUndefined();
    expect(composeTransition("x", true)).toBe("x");
  });
});
