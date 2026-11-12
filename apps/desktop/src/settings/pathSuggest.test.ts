import { describe, expect, it } from "vitest";
import { shouldSuggest } from "./pathSuggest";

describe("shouldSuggest", () => {
  it("sugiere mientras escribes un nombre", () => {
    expect(shouldSuggest("tema4")).toBe(true);
    expect(shouldSuggest("apuntes bd")).toBe(true);
    expect(shouldSuggest("teoría")).toBe(true);
  });

  it("no sugiere con una ruta completa", () => {
    expect(shouldSuggest(String.raw`C:\Users\Darío\tema4.pdf`)).toBe(false);
    expect(shouldSuggest("d:/apuntes")).toBe(false);
    expect(shouldSuggest(String.raw`\\servidor\compartido`)).toBe(false);
  });

  it("no sugiere con menos de dos letras", () => {
    expect(shouldSuggest("")).toBe(false);
    expect(shouldSuggest(" a ")).toBe(false);
    expect(shouldSuggest("-_")).toBe(false);
  });
});
