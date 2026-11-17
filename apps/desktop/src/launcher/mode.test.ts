import { describe, expect, it } from "vitest";
import { addStep } from "./mode";

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
