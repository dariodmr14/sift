import { describe, expect, it } from "vitest";

// Test mínimo para que `npm test` y la CI tengan algo que ejecutar.
// Se sustituye por tests reales en la Fase 1.
describe("smoke", () => {
  it("vitest funciona", () => {
    expect(1 + 1).toBe(2);
  });
});
