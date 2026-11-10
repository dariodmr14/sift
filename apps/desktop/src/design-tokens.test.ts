// El sistema de diseño solo funciona si se usa: este test falla si un componente pinta
// con colores escritos a mano en vez de con los tokens de src/index.css.
import { readdirSync, readFileSync, statSync } from "node:fs";
import { join, relative } from "node:path";
import { describe, expect, it } from "vitest";

function components(dir: string): string[] {
  return readdirSync(dir).flatMap((name) => {
    const path = join(dir, name);
    if (statSync(path).isDirectory()) return components(path);
    return path.endsWith(".tsx") ? [path] : [];
  });
}

/** Colores literales: hex, rgb()/hsl(), o la paleta de Tailwind (`text-red-400`…). */
const LITERAL =
  /#[0-9a-fA-F]{3,8}\b|\b(?:rgb|rgba|hsl|hsla)\(|\b(?:text|bg|border|ring|fill|stroke|from|to|via|outline|decoration|shadow|accent)-(?:black|white|slate|gray|zinc|neutral|stone|red|orange|amber|yellow|lime|green|emerald|teal|cyan|sky|blue|indigo|violet|purple|fuchsia|pink|rose)\b/;

describe("tokens de diseño", () => {
  const files = components(join(import.meta.dirname, "."));

  it("hay componentes que revisar", () => {
    expect(files.length).toBeGreaterThan(5);
  });

  it.each(files.map((f) => [relative(import.meta.dirname, f), f]))(
    "%s usa solo tokens",
    (_name, path) => {
      const offending = readFileSync(path, "utf8")
        .split("\n")
        .map((line, i) => `${i + 1}: ${line.trim()}`)
        .filter((line) => LITERAL.test(line));
      expect(offending).toEqual([]);
    },
  );
});
