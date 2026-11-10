import { describe, expect, it } from "vitest";
import { matchBuiltins } from "./builtins";

const ids = (q: string) => matchBuiltins(q).map((c) => c.id);

describe("matchBuiltins", () => {
  it("encuentra Ajustes por su nombre y por sinónimos", () => {
    expect(ids("ajustes")).toEqual(["settings"]);
    expect(ids("Configuración")).toEqual(["settings"]);
    expect(ids("prefer")).toEqual(["settings"]);
    expect(ids("settings")).toEqual(["settings"]);
  });

  it("acepta el principio de la palabra, sin tildes ni mayúsculas", () => {
    expect(ids("AJU")).toEqual(["settings"]);
    expect(ids("config")).toEqual(["settings"]);
  });

  it("no encuentra por trozos del medio de una palabra", () => {
    expect(ids("tes")).toEqual([]);
  });

  it("no muestra nada con la búsqueda vacía o de una letra", () => {
    expect(ids("")).toEqual([]);
    expect(ids("a")).toEqual([]);
  });

  it("encuentra Crear Espacio", () => {
    expect(ids("crear")).toEqual(["newSpace"]);
    expect(ids("nuevo espacio")).toEqual(["newSpace"]);
  });

  it("todas las palabras tienen que encajar", () => {
    expect(ids("ajustes sift")).toEqual(["settings"]);
    expect(ids("ajustes zzz")).toEqual([]);
  });
});
