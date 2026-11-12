import { describe, expect, it } from "vitest";
import { detectFilter, TYPE_FILTERS } from "./filters";

describe("detectFilter", () => {
  it("una palabra clave y un espacio activan el filtro", () => {
    expect(detectFilter("pdf contrato")).toEqual({
      filter: TYPE_FILTERS[0],
      rest: "contrato",
    });
    expect(detectFilter("pdf ")?.rest).toBe("");
  });

  it("sin tildes ni mayúsculas que importen", () => {
    expect(detectFilter("Código árbol")?.filter.id).toBe("code");
    expect(detectFilter("FOTOS playa")?.filter.id).toBe("image");
    expect(detectFilter("vídeo boda")?.filter.id).toBe("video");
    expect(detectFilter("diapositivas tema 3")?.filter.id).toBe("slides");
  });

  it("mientras escribes la palabra (sin espacio) no salta", () => {
    expect(detectFilter("pdf")).toBeNull();
    expect(detectFilter("codigos")).toBeNull();
  });

  it("solo al principio, y no con palabras normales", () => {
    expect(detectFilter("contrato pdf ")).toBeNull();
    expect(detectFilter("notas del examen")).toBeNull();
    expect(detectFilter("documento de requisitos")).toBeNull();
  });

  it("cada filtro tiene palabras sin tildes y distintas", () => {
    const all = TYPE_FILTERS.flatMap((f) => f.keywords);
    expect(new Set(all).size).toBe(all.length);
    for (const k of all) expect(k).toBe(k.normalize("NFD").toLowerCase());
    for (const k of all) expect(k).not.toMatch(/\p{Diacritic}/u);
  });
});
