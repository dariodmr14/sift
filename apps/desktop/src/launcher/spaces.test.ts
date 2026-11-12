import { describe, expect, it } from "vitest";
import { score } from "./spaces";

const bd = { name: "Bases de datos", aliases: ["bd", "SQL"] };

describe("score", () => {
  it("exacto > empieza por > principio de palabras > contiene", () => {
    expect(score("bd", bd)).toBe(4);
    expect(score("BASES D", bd)).toBe(3);
    expect(score("dat bas", bd)).toBe(2);
    expect(score("atos", bd)).toBe(1);
    expect(score("zzz", bd)).toBe(0);
  });

  it("no distingue mayúsculas ni tildes", () => {
    expect(score("báses dé datos", bd)).toBe(4);
    expect(score("sql", bd)).toBe(4);
  });

  it("la búsqueda vacía no puntúa", () => {
    expect(score("  ", bd)).toBe(0);
  });
});
