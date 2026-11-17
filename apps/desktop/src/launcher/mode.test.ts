import { describe, expect, it } from "vitest";
import { addStep, panelKey } from "./mode";

describe("qué ocupa la barra", () => {
  it("los paneles mandan sobre la lista", () => {
    const base = { isEmpty: true, hasItems: true };
    expect(panelKey({ ...base, mode: "running" })).toBe("running");
    expect(panelKey({ ...base, mode: "result" })).toBe("result");
  });
  it("en la lista: vacía, con resultados o sin resultados", () => {
    expect(panelKey({ mode: "list", isEmpty: true, hasItems: false })).toBe(
      "empty",
    );
    expect(panelKey({ mode: "list", isEmpty: false, hasItems: true })).toBe(
      "list",
    );
    expect(panelKey({ mode: "list", isEmpty: false, hasItems: false })).toBe(
      "none",
    );
  });
});

describe("pasos de un Espacio en marcha", () => {
  it("cada paso nuevo se apunta una vez, aunque una espera avise muchas", () => {
    let steps = addStep([], { step: 1, total: 3, label: "Abrir carpeta" });
    steps = addStep(steps, { step: 2, total: 3, label: "Esperar a la base" });
    steps = addStep(steps, {
      step: 2,
      total: 3,
      label: "Esperar a la base",
      waited: 4,
    });
    steps = addStep(steps, { step: 3, total: 3, label: "Abrir VS Code" });
    expect(steps.map((s) => s.step)).toEqual([1, 2, 3]);
  });
});
