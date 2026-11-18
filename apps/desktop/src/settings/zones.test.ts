import { describe, expect, it } from "vitest";
import type { MonitorInfo } from "../bindings";
import { monitorOptions, ZONE_GROUPS, ZONES } from "./zones";

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

describe("ZONE_GROUPS", () => {
  it("cada zona sale una vez, y «No colocar» va la primera", () => {
    const all = ZONE_GROUPS.flatMap((g) => g.zones);
    expect(all[0]).toBeNull();
    const zones = all.filter((z) => z !== null);
    expect([...zones].sort()).toEqual(ZONES.map((z) => z.id).sort());
    expect(ZONE_GROUPS.map((g) => g.label)).toEqual([
      "General",
      "Mitades",
      "Tercios",
      "Cuartos",
    ]);
  });
});

describe("monitorOptions", () => {
  it("con una pantalla: su número, su modelo, cuál es la principal y la del ratón", () => {
    const options = monitorOptions([screen("DELL U2720Q", 0, true)], {
      kind: "primary",
    });
    expect(options.map((o) => [o.value, o.label])).toEqual([
      ["primary", "Pantalla 1 · DELL U2720Q (principal)"],
      ["underMouse", "Donde esté el ratón"],
    ]);
  });

  it("sin modelo conocido no repite «Pantalla»", () => {
    const options = monitorOptions([screen("Pantalla 2", 0, true)], {
      kind: "primary",
    });
    expect(options[0].label).toBe("Pantalla 1 (principal)");
  });

  it("con dos, numeradas de izquierda a derecha; la principal se guarda como «la principal»", () => {
    const options = monitorOptions(
      [screen("LG", 1920, false), screen("DELL", 0, true)],
      { kind: "primary" },
    );
    expect(options.map((o) => o.label)).toEqual([
      "Pantalla 1 · DELL (principal)",
      "Pantalla 2 · LG",
      "Donde esté el ratón",
    ]);
    expect(options[0].choice).toEqual({ kind: "primary" });
    expect(options[1].choice).toEqual({
      kind: "specific",
      name: "LG",
      left: 1920,
      top: 0,
    });
  });

  it("una principal guardada como pantalla concreta se respeta", () => {
    const saved = { kind: "specific" as const, name: "DELL", left: 0, top: 0 };
    const options = monitorOptions([screen("DELL", 0, true)], saved);
    expect(options[0]).toMatchObject({
      label: "Pantalla 1 · DELL (principal)",
      choice: saved,
    });
    expect(options.some((o) => o.label.includes("no conectada"))).toBe(false);
  });

  it("una guardada que ya no está conectada no se pierde", () => {
    const saved = { kind: "specific" as const, name: "LG", left: 1920, top: 0 };
    const options = monitorOptions([screen("DELL", 0, true)], saved);
    expect(options[options.length - 1].label).toBe(
      "LG (no conectada: se usará la principal)",
    );
  });
});
