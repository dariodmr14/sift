import { describe, expect, it } from "vitest";
import { mixResults } from "./mix";

const space = (name: string, aliases: string[] = [], boost = 1) => ({
  name,
  aliases,
  boost,
});
const file = (name: string, score: number, boost = 1) => ({
  name,
  score,
  boost,
});

/** Nombres en orden, con «E:» delante de los Espacios. */
function names(
  query: string,
  spaces: ReturnType<typeof space>[],
  files: ReturnType<typeof file>[],
) {
  return mixResults(query, spaces, files).map((m) =>
    m.kind === "space" ? `E:${m.item.name}` : m.item.name,
  );
}

describe("mixResults", () => {
  it("con la búsqueda vacía, todos los Espacios en el orden de Ajustes", () => {
    expect(
      names("", [space("B", [], 1.2), space("A")], [file("x", 1)]),
    ).toEqual(["E:B", "E:A"]);
  });

  it("un nombre o alias exacto va primero aunque no se use", () => {
    const files = [file("muy usado", 0.05, 1.25), file("otro", 0.04)];
    expect(names("bd", [space("Bases de datos", ["bd"])], files)).toEqual([
      "E:Bases de datos",
      "muy usado",
      "otro",
    ]);
  });

  it("entre varios exactos decide la frecencia", () => {
    const spaces = [space("Uni", ["u"]), space("Uni", ["x"], 1.2)];
    const out = mixResults("uni", spaces, []);
    expect(out.map((m) => m.item.boost)).toEqual([1.2, 1]);
  });

  it("un Espacio parcial sin uso empata con el mejor archivo sin uso y va delante", () => {
    const files = [file("apuntes.pdf", 0.05), file("practica.sql", 0.03)];
    expect(names("bases d", [space("Bases de datos")], files)).toEqual([
      "E:Bases de datos",
      "apuntes.pdf",
      "practica.sql",
    ]);
  });

  it("parcial: un archivo muy usado gana a un Espacio sin uso", () => {
    const files = [file("apuntes.pdf", 0.05 * 1.2, 1.2), file("otro", 0.03)];
    expect(names("bases d", [space("Bases de datos")], files)).toEqual([
      "apuntes.pdf",
      "E:Bases de datos",
      "otro",
    ]);
  });

  it("parcial: un Espacio muy usado gana a un archivo sin uso", () => {
    const files = [file("apuntes.pdf", 0.05), file("otro", 0.03)];
    expect(names("bases d", [space("Bases de datos", [], 1.2)], files)).toEqual(
      ["E:Bases de datos", "apuntes.pdf", "otro"],
    );
  });

  it("cuanto peor encaja, más abajo entra", () => {
    const files = [
      file("a", 0.05),
      file("b", 0.045),
      file("c", 0.035),
      file("d", 0.02),
    ];
    // «atos» solo aparece dentro de una palabra: vale la mitad que el mejor archivo.
    expect(names("atos", [space("Bases de datos")], files)).toEqual([
      "a",
      "b",
      "c",
      "E:Bases de datos",
      "d",
    ]);
  });

  it("los archivos nunca cambian de orden entre sí", () => {
    // Llegan en un orden que no es el de su puntuación (regla del nombre en Rust).
    const files = [file("nombre", 0.01), file("contenido", 0.05)];
    const out = names("bases d", [space("Bases de datos")], files);
    expect(out.filter((n) => !n.startsWith("E:"))).toEqual([
      "nombre",
      "contenido",
    ]);
  });

  it("encuentra por alias", () => {
    const spaces = [space("TFG", ["trabajo fin de grado"]), space("Redes")];
    expect(names("trabajo", spaces, [])).toEqual(["E:TFG"]);
  });

  it("a igualdad, el orden de Ajustes", () => {
    const spaces = [
      space("Datos B"),
      space("Datos A"),
      space("Otra", ["datos"]),
    ];
    expect(names("datos", spaces, [])).toEqual([
      "E:Otra",
      "E:Datos B",
      "E:Datos A",
    ]);
  });

  it("los Espacios que no encajan no salen", () => {
    expect(names("redes", [space("Bases de datos")], [file("x", 1)])).toEqual([
      "x",
    ]);
  });
});
