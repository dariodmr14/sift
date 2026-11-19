import { describe, expect, it } from "vitest";
import { addStep, panelKey, placing, setPlacement, type RunStep } from "./mode";

describe("qué ocupa la barra", () => {
  it("los paneles mandan sobre la lista", () => {
    const base = { isEmpty: true, hasItems: true, preview: true };
    expect(panelKey({ ...base, mode: "running" })).toBe("running");
    expect(panelKey({ ...base, mode: "result" })).toBe("result");
  });
  it("en la lista: vacía, con resultados o sin resultados", () => {
    expect(
      panelKey({
        mode: "list",
        isEmpty: true,
        hasItems: false,
        preview: false,
      }),
    ).toBe("empty");
    expect(
      panelKey({
        mode: "list",
        isEmpty: false,
        hasItems: true,
        preview: false,
      }),
    ).toBe("list");
    expect(
      panelKey({
        mode: "list",
        isEmpty: false,
        hasItems: false,
        preview: false,
      }),
    ).toBe("none");
  });
});

describe("vista previa", () => {
  it("ocupa el lugar de la lista, y la lista vuelve al cerrarla", () => {
    const base = { mode: "list" as const, isEmpty: false, hasItems: true };
    expect(panelKey({ ...base, preview: true })).toBe("preview");
    expect(panelKey({ ...base, preview: false })).toBe("list");
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

describe("colocar ventanas en segundo plano", () => {
  it("cada aviso marca su paso, aunque llegue después de otros", () => {
    let steps: RunStep[] = [
      { step: 1, label: "Abrir Eclipse" },
      { step: 2, label: "Abrir Opera" },
    ];
    steps = setPlacement(steps, { step: 1, state: "placing" });
    steps = setPlacement(steps, { step: 2, state: "placing" });
    steps = setPlacement(steps, { step: 2, state: "placed" });
    expect(steps.map((s) => s.placement)).toEqual(["placing", "placed"]);
    expect(placing(steps)).toBe(true);
    steps = setPlacement(steps, { step: 1, state: "notPlaced" });
    expect(placing(steps)).toBe(false);
  });
});
