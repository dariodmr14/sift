import { describe, expect, it } from "vitest";
import { progressText, summarize, type Outcome } from "./runSummary";

const ok = (index: number, label: string): Outcome => ({
  index,
  label,
  error: null,
  ran: true,
});

describe("summarize", () => {
  it("si todo fue bien, no hace falta enseñar nada", () => {
    const s = summarize([ok(1, "Abrir VS Code"), ok(2, "Abrir web")], false);
    expect(s.needsAttention).toBe(false);
  });

  it("un fallo suelto: lo dice, y el resto se abrió", () => {
    const s = summarize(
      [
        { index: 1, label: "Abrir web", error: "sin navegador", ran: true },
        ok(2, "Abrir carpeta"),
      ],
      false,
    );
    expect(s.needsAttention).toBe(true);
    expect(s.headline).toBe("Algo no se pudo abrir");
    expect(s.lines.map((l) => l.state)).toEqual(["failed", "ok"]);
  });

  it("una espera que para: en qué paso, y lo que no se ejecutó", () => {
    const s = summarize(
      [
        ok(1, "Ejecutar docker compose up -d"),
        {
          index: 2,
          label: "Esperar a que responda http://localhost:8080",
          error: "http://localhost:8080 no respondió en 60 s",
          ran: true,
        },
        {
          index: 3,
          label: "Abrir http://localhost:8080",
          error: null,
          ran: false,
        },
      ],
      false,
    );
    expect(s.headline).toBe(
      "Se paró en el paso 2: lo que ya se abrió sigue abierto",
    );
    expect(s.lines.map((l) => l.state)).toEqual(["ok", "failed", "notRun"]);
    expect(s.lines[1].detail).toContain("no respondió");
  });

  it("cancelar: lo hecho, lo que se estaba esperando y lo que no se ejecutó", () => {
    const s = summarize(
      [
        ok(1, "Abrir VS Code"),
        {
          index: 2,
          label: "Esperar a que responda http://localhost:8080",
          error: "cancelado mientras esperaba",
          ran: false,
        },
        { index: 3, label: "Abrir web", error: null, ran: false },
      ],
      true,
    );
    expect(s.headline).toBe("Cancelado: lo que ya se abrió sigue abierto");
    expect(s.lines.map((l) => l.state)).toEqual(["ok", "cancelled", "notRun"]);
  });
});

describe("progressText", () => {
  it("un paso normal y una espera con sus segundos", () => {
    expect(
      progressText({ step: 1, total: 3, label: "Abrir VS Code", waited: null }),
    ).toBe("Paso 1 de 3 · Abrir VS Code…");
    expect(
      progressText({
        step: 2,
        total: 3,
        label: "Esperar a que responda http://localhost:8080",
        waited: 12,
      }),
    ).toBe(
      "Paso 2 de 3 · Esperando a que responda http://localhost:8080… 12 s",
    );
  });
});
