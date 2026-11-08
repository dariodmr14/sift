import { describe, expect, it } from "vitest";
import { MOCK_RESULTS, filterResults, normalize } from "./results";

const titles = (query: string) =>
  filterResults(query, MOCK_RESULTS).map((r) => r.title);

describe("normalize", () => {
  it("quita tildes y mayúsculas", () => {
    expect(normalize("Normalización PRÁCTICA")).toBe("normalizacion practica");
  });
});

describe("filterResults", () => {
  it("con la búsqueda vacía devuelve todo", () => {
    expect(filterResults("   ", MOCK_RESULTS)).toHaveLength(
      MOCK_RESULTS.length,
    );
  });

  it("no distingue tildes ni mayúsculas", () => {
    expect(titles("NORMALIZACION")).toEqual(["Tema 4 - Normalización.pdf"]);
  });

  it("exige todas las palabras, en cualquier orden", () => {
    expect(titles("pdf tema")).toEqual([
      "Tema 4 - Normalización.pdf",
      "Tema 5 - Transacciones.pdf",
    ]);
    expect(titles("tema transacciones")).toEqual([
      "Tema 5 - Transacciones.pdf",
    ]);
  });

  it("busca también en el subtítulo", () => {
    expect(titles("campusvirtual")).toEqual(["Campus Virtual"]);
  });

  it("sin coincidencias devuelve una lista vacía", () => {
    expect(titles("zzz")).toEqual([]);
  });
});
