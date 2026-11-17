import { describe, expect, it } from "vitest";
import { caretAtEnd, previewReason, reasonLabel } from "./preview";

const seg = (text: string, hit = false) => ({ text, hit });

describe("por qué sale un resultado", () => {
  it("con palabras encontradas en el texto: «Contiene …»", () => {
    const r = previewReason({
      snippet: [
        seg("la "),
        seg("clave", true),
        seg(" "),
        seg("primaria", true),
      ],
      byMeaning: false,
    });
    expect(r).toEqual({ kind: "contains", words: ["clave", "primaria"] });
    expect(reasonLabel(r)).toBe("Contiene «clave», «primaria»");
  });

  it("por significado gana a lo demás", () => {
    const r = previewReason({
      snippet: [seg("clave", true)],
      byMeaning: true,
    });
    expect(r).toEqual({ kind: "meaning" });
    expect(reasonLabel(r)).toBe("Trata de lo mismo");
  });

  it("sin coincidencias: por el nombre", () => {
    const r = previewReason({ snippet: [seg("nada")], byMeaning: false });
    expect(r).toEqual({ kind: "name" });
    expect(reasonLabel(r)).toBe("Por el nombre");
    expect(previewReason({ snippet: [], byMeaning: false }).kind).toBe("name");
  });

  it("palabras únicas sin distinguir mayúsculas ni tildes, con la primera forma", () => {
    const r = previewReason({
      snippet: [
        seg("Normalización", true),
        seg(" y "),
        seg("normalizacion", true),
        seg(" "),
        seg("NORMALIZACIÓN", true),
        seg("tabla", true),
      ],
      byMeaning: false,
    });
    expect(r).toEqual({ kind: "contains", words: ["Normalización", "tabla"] });
  });

  it("como mucho tres palabras, en orden de aparición", () => {
    const r = previewReason({
      snippet: ["uno", "dos", "tres", "cuatro"].map((w) => seg(w, true)),
      byMeaning: false,
    });
    expect(r).toEqual({ kind: "contains", words: ["uno", "dos", "tres"] });
  });
});

describe("cursor al final de la caja", () => {
  it("solo si el cursor está al final y no hay selección", () => {
    expect(caretAtEnd("hola", 4, 4)).toBe(true);
    expect(caretAtEnd("hola", 2, 2)).toBe(false);
    expect(caretAtEnd("hola", 0, 4)).toBe(false);
    expect(caretAtEnd("", 0, 0)).toBe(true);
    expect(caretAtEnd("hola", null, null)).toBe(false);
  });
});
