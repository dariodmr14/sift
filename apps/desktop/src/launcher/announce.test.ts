import { describe, expect, it } from "vitest";
import { TYPE_FILTERS } from "./filters";
import { resultsAnnouncement } from "./announce";

const pdf = TYPE_FILTERS[0];

describe("resultsAnnouncement", () => {
  it("barra vacía: no anuncia nada", () => {
    expect(
      resultsAnnouncement({ query: "", filter: null, count: 5, empty: true }),
    ).toBe("");
  });

  it("filtro sin consulta", () => {
    expect(
      resultsAnnouncement({ query: " ", filter: pdf, count: 0, empty: false }),
    ).toBe("Buscando solo en tus PDF");
  });

  it("sin resultados, con y sin filtro", () => {
    expect(
      resultsAnnouncement({ query: "a", filter: null, count: 0, empty: false }),
    ).toBe("Sin resultados");
    expect(
      resultsAnnouncement({ query: "a", filter: pdf, count: 0, empty: false }),
    ).toBe("Sin resultados en tus PDF");
  });

  it("uno y varios, con separador de miles", () => {
    expect(
      resultsAnnouncement({ query: "a", filter: null, count: 1, empty: false }),
    ).toBe("1 resultado");
    expect(
      resultsAnnouncement({ query: "a", filter: pdf, count: 1, empty: false }),
    ).toBe("1 resultado en tus PDF");
    expect(
      resultsAnnouncement({ query: "a", filter: null, count: 7, empty: false }),
    ).toBe("7 resultados");
    expect(
      resultsAnnouncement({
        query: "a",
        filter: pdf,
        count: 12345,
        empty: false,
      }),
    ).toBe(`${(12345).toLocaleString("es-ES")} resultados en tus PDF`);
  });
});
