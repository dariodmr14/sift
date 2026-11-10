import { describe, expect, it } from "vitest";
import { matchSpaces, score } from "./spaces";

const bd = { name: "Bases de datos", aliases: ["bd", "SQL"] };
const redes = { name: "Redes", aliases: ["networking"] };
const tfg = { name: "TFG", aliases: ["trabajo fin de grado"] };
const all = [redes, bd, tfg];

const names = (q: string) => matchSpaces(q, all).map((s) => s.name);

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

describe("matchSpaces", () => {
  it("con la búsqueda vacía devuelve todos en el orden del usuario", () => {
    expect(names("")).toEqual(["Redes", "Bases de datos", "TFG"]);
  });

  it("encuentra por alias", () => {
    expect(names("trabajo")).toEqual(["TFG"]);
    expect(names("network")).toEqual(["Redes"]);
  });

  it("ordena por calidad y, a igualdad, por el orden del usuario", () => {
    const a = { name: "Datos A", aliases: [] };
    const b = { name: "Datos B", aliases: [] };
    const exact = { name: "Otra", aliases: ["datos"] };
    expect(matchSpaces("datos", [a, b, exact]).map((s) => s.name)).toEqual([
      "Otra",
      "Datos A",
      "Datos B",
    ]);
  });

  it("sin coincidencias, lista vacía", () => {
    expect(names("zzz")).toEqual([]);
  });
});
