import { describe, expect, it } from "vitest";
import {
  clampStep,
  firstSpaceDraft,
  hotkeyText,
  INTRO_STEPS,
  stepLabel,
} from "./introSteps";

describe("introducción", () => {
  it("cinco pasos como mucho, cada uno con su título", () => {
    expect(INTRO_STEPS.length).toBeLessThanOrEqual(5);
    expect(stepLabel(1)).toBe("Paso 2 de 5: Dónde buscar");
    expect(clampStep(-1)).toBe(0);
    expect(clampStep(9)).toBe(4);
  });

  it("el atajo se dice como en un teclado español", () => {
    expect(hotkeyText("Alt+Space")).toBe("Alt+Espacio");
  });

  it("el primer Espacio: carpetas y web; sin nombre o sin acciones, nada", () => {
    expect(firstSpaceDraft("  Uni ", ["C:\\Uni"], "campus.es")).toEqual({
      name: "Uni",
      aliases: [],
      icon: { kind: "auto" },
      actions: [
        { kind: "openFolder", path: "C:\\Uni" },
        { kind: "openUrl", url: "https://campus.es" },
      ],
    });
    expect(
      firstSpaceDraft("Uni", [], "http://localhost:3000")?.actions,
    ).toEqual([{ kind: "openUrl", url: "http://localhost:3000" }]);
    expect(firstSpaceDraft("", ["C:\\Uni"], "")).toBeNull();
    expect(firstSpaceDraft("Uni", [], " ")).toBeNull();
  });
});
