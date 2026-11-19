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

  // Las opciones de los desplegables nativos no heredan el fondo del tema: sin esta regla
  // salían en blanco sobre blanco con el tema oscuro (FEEDBACK 2026-11-19).
  it("las opciones de los desplegables tienen fondo y texto del tema", () => {
    const css = readFileSync(join(import.meta.dirname, "index.css"), "utf8");
    expect(css).toMatch(
      /option,\s*optgroup\s*\{\s*background-color:\s*var\(--surface-solid\);\s*color:\s*var\(--fg\);/,
    );
  });

  // El indicador de carga (SiftLoader): con «reducir movimiento», las barras quietas y
  // solo un pulso de opacidad.
  it("con «reducir movimiento» el indicador de carga solo late", () => {
    const css = readFileSync(join(import.meta.dirname, "index.css"), "utf8");
    const reduce = css.slice(css.lastIndexOf("prefers-reduced-motion"));
    expect(reduce).toMatch(/\.sift-loader > span \{\s*animation: none;/);
    expect(reduce).toMatch(/\.sift-loader \{\s*animation: sift-pulse/);
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
