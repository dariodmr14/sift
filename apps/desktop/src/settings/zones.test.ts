import { describe, expect, it } from "vitest";
import type { MonitorInfo } from "../bindings";
import { monitorOptions, ZONE_ROWS, ZONES } from "./zones";

const screen = (name: string, left: number, primary: boolean): MonitorInfo => {
  const bounds = { left, top: 0, right: left + 1920, bottom: 1080 };
  return { name, bounds, work: bounds, primary };
};

describe("ZONES", () => {
  it("las 14 zonas, sin repetir y dentro de la pantalla", () => {
    expect(ZONES).toHaveLength(14);
    expect(new Set(ZONES.map((z) => z.id)).size).toBe(14);
    for (const { sixths } of ZONES) {
      const [l, t, w, h] = sixths;
      expect(l + w).toBeLessThanOrEqual(6);
      expect(t + h).toBeLessThanOrEqual(6);
    }
  });
});

describe("ZONE_ROWS", () => {
  it("cada zona sale una vez en las filas", () => {
    const all = ZONE_ROWS.flat();
    expect([...all].sort()).toEqual(ZONES.map((z) => z.id).sort());
  });
});

describe("monitorOptions", () => {
  it("con una pantalla, solo la principal y la del ratón", () => {
    const options = monitorOptions([screen("Pantalla 1", 0, true)], {
      kind: "primary",
    });
    expect(options.map((o) => o.value)).toEqual(["primary", "underMouse"]);
  });

  it("con dos, cada una con dónde está, de izquierda a derecha", () => {
    const options = monitorOptions(
      [screen("Pantalla 1", 1920, false), screen("Pantalla 2", 0, true)],
      { kind: "primary" },
    );
    expect(options.slice(2).map((o) => o.label)).toEqual([
      "Pantalla 2, la de la izquierda (principal)",
      "Pantalla 1, la de la derecha",
    ]);
    expect(options[3].choice).toEqual({
      kind: "specific",
      name: "Pantalla 1",
      left: 1920,
      top: 0,
    });
  });

  it("una guardada que ya no está conectada no se pierde", () => {
    const saved = { kind: "specific" as const, name: "LG", left: 1920, top: 0 };
    const options = monitorOptions([screen("Pantalla 1", 0, true)], saved);
    expect(options[options.length - 1].label).toBe(
      "LG (no conectada: se usará la principal)",
    );
  });
});
