import { describe, expect, it } from "vitest";
import { isLatest, sectionStarts, shortFolder } from "./files";

describe("shortFolder", () => {
  it("deja la carpeta entera si es corta", () => {
    expect(shortFolder(String.raw`C:\Uni\tema4.md`)).toBe(String.raw`C:\Uni`);
  });

  it("acorta las rutas largas por el principio", () => {
    expect(
      shortFolder(String.raw`C:\Users\Darío\Documents\Uni\BD\Teoría\tema4.pdf`),
    ).toBe(String.raw`…\Uni\BD\Teoría`);
  });

  it("acepta barras normales", () => {
    expect(shortFolder("C:/a/b.txt")).toBe(String.raw`C:\a`);
  });
});

describe("sectionStarts", () => {
  it("sin cabeceras si una sección sale en dos trozos", () => {
    expect(sectionStarts(["spaces", "files", "spaces", "files"]).size).toBe(0);
  });

  it("marca el primer elemento de cada sección", () => {
    const starts = sectionStarts([
      "spaces",
      "spaces",
      "files",
      "files",
      "commands",
    ]);
    expect([...starts]).toEqual([0, 2, 4]);
  });

  it("lista vacía", () => {
    expect(sectionStarts([]).size).toBe(0);
  });
});

describe("isLatest", () => {
  it("descarta respuestas atrasadas", () => {
    expect(isLatest(3, 3)).toBe(true);
    expect(isLatest(2, 3)).toBe(false);
  });
});
