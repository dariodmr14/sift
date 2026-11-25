// El vidrio tiene que dejar leer el texto sobre CUALQUIER escritorio, no solo sobre uno
// neutro (FEEDBACK 2026-11-25: en tema claro, sobre el fondo azul de Windows, la
// introducción casi no se leía). Este test toma los colores reales de src/index.css y el
// tinte del Acrylic de materials.rs, compone el vidrio sobre los peores fondos y exige el
// contraste mínimo de WCAG AA (4,5:1) a todo el texto, también sobre una fila seleccionada.
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { describe, expect, it } from "vitest";

type Rgb = [number, number, number];

const css = readFileSync(join(import.meta.dirname, "index.css"), "utf8");
const tauri = join(import.meta.dirname, "..", "src-tauri", "src");
const materialsRs = readFileSync(join(tauri, "materials.rs"), "utf8");
const appearanceRs = readFileSync(join(tauri, "appearance.rs"), "utf8");

/** El contenido del bloque n.º `nth` (desde 0) con este selector exacto. */
function block(selector: string, nth = 0): string {
  let start = -1;
  for (let i = 0; i <= nth; i++) {
    start = css.indexOf(`\n${selector} {\n`, start + 1);
  }
  expect(start, `no se encuentra «${selector}» en index.css`).toBeGreaterThan(
    -1,
  );
  return css.slice(start, css.indexOf("\n}", start));
}

/** `--name: rgb(r g b)` o `rgb(r g b / a)` (Prettier a veces lo parte en varias líneas). */
function token(text: string, name: string): { rgb: Rgb; alpha: number } {
  const m = new RegExp(
    `--${name}:\\s*rgb\\(\\s*(\\d+)\\s+(\\d+)\\s+(\\d+)\\s*(?:/\\s*([\\d.]+))?\\s*\\)`,
  ).exec(text);
  expect(m, `falta --${name}`).not.toBeNull();
  const [, r, g, b, a] = m!;
  return { rgb: [Number(r), Number(g), Number(b)], alpha: a ? Number(a) : 1 };
}

function lin(c: number): number {
  const s = c / 255;
  return s <= 0.03928 ? s / 12.92 : ((s + 0.055) / 1.055) ** 2.4;
}
function luminance([r, g, b]: Rgb): number {
  return 0.2126 * lin(r) + 0.7152 * lin(g) + 0.0722 * lin(b);
}
function contrast(a: Rgb, b: Rgb): number {
  const [x, y] = [luminance(a), luminance(b)];
  return (Math.max(x, y) + 0.05) / (Math.min(x, y) + 0.05);
}
/** `top` con opacidad `alpha` encima de `bottom` (como compone Windows y el navegador). */
function over(top: Rgb, alpha: number, bottom: Rgb): Rgb {
  return [0, 1, 2].map((i) => top[i] * alpha + bottom[i] * (1 - alpha)) as Rgb;
}

/** Tinte del Acrylic (`tint()` en materials.rs): color y alfa sobre 255. */
function acrylicTint(theme: "Dark" | "Light"): { rgb: Rgb; alpha: number } {
  const m = new RegExp(
    `Theme::${theme} => \\((\\d+), (\\d+), (\\d+), (\\d+)\\)`,
  ).exec(materialsRs.slice(materialsRs.indexOf("pub fn tint(")));
  expect(m, `falta el tinte ${theme} en materials.rs`).not.toBeNull();
  const [, r, g, b, a] = m!.map(Number);
  return { rgb: [r, g, b], alpha: a / 255 };
}

/** Escritorios de prueba: los extremos, colores saturados y el fondo azul de Windows 10. */
const BACKDROPS: Rgb[] = [
  [0, 0, 0],
  [255, 255, 255],
  [128, 128, 128],
  [255, 0, 0],
  [0, 255, 0],
  [0, 0, 255],
  [255, 255, 0],
  [0, 255, 255],
  [255, 0, 255],
  [0, 120, 215],
  [90, 200, 250],
];

const TEXT = ["fg", "muted", "danger", "success", "accent-ink"];

const themes = [
  {
    name: "oscuro",
    tokens: block(":root"),
    glass: token(block(':root[data-glass="true"]'), "surface"),
    tint: acrylicTint("Dark"),
    selected: token(block(":root"), "selected"),
    selectedColor: [255, 255, 255] as Rgb,
    rustWorst: "GLASS_WORST_DARK",
  },
  {
    name: "claro",
    tokens: block(':root[data-theme="light"]'),
    glass: token(
      block(':root[data-glass="true"][data-theme="light"]'),
      "surface",
    ),
    tint: acrylicTint("Light"),
    selected: token(block(':root[data-theme="light"]'), "selected"),
    selectedColor: [0, 0, 0] as Rgb,
    rustWorst: "GLASS_WORST_LIGHT",
  },
];

/** El fondo que queda: escritorio → tinte del Acrylic → capa `--surface` de la UI. */
function glassOver(t: (typeof themes)[number], backdrop: Rgb): Rgb {
  return over(
    t.glass.rgb,
    t.glass.alpha,
    over(t.tint.rgb, t.tint.alpha, backdrop),
  );
}

describe.each(themes)("vidrio en tema $name", (t) => {
  const solid = token(t.tokens, "surface-solid").rgb;
  // Sin vidrio y con él, y con y sin una fila seleccionada encima.
  const plain: [string, Rgb][] = ([["opaco", solid]] as [string, Rgb][]).concat(
    BACKDROPS.map((b): [string, Rgb] => [
      `vidrio sobre ${b.join(" ")}`,
      glassOver(t, b),
    ]),
  );
  const backgrounds: [string, Rgb][] = plain.flatMap(
    ([label, bg]): [string, Rgb][] => [
      [label, bg],
      [
        `${label}, fila seleccionada`,
        over(t.selectedColor, t.selected.alpha, bg),
      ],
    ],
  );

  it.each(TEXT)("--%s se lee (4,5:1) sobre cualquier escritorio", (name) => {
    const ink = token(t.tokens, name).rgb;
    const failing = backgrounds
      .map(([label, bg]) => [label, contrast(ink, bg)] as const)
      .filter(([, c]) => c < 4.5)
      .map(([label, c]) => `${label}: ${c.toFixed(2)}`);
    expect(failing).toEqual([]);
  });

  it("el diálogo flotante (--glass) cubre lo mismo que la ventana", () => {
    const tint = t.tint.alpha;
    const total = 1 - (1 - tint) * (1 - t.glass.alpha);
    const dialog = token(
      t.name === "claro"
        ? block(':root[data-theme="light"]', 1)
        : block(":root", 1),
      "glass",
    );
    expect(Math.abs(dialog.alpha - total)).toBeLessThan(0.01);
  });

  // Rust oscurece o aclara el acento del usuario hasta leerse sobre el peor vidrio
  // (`accent_ink` en appearance.rs): ese peor fondo tiene que ser el de aquí.
  it(`${t.rustWorst} en appearance.rs es el peor fondo del vidrio`, () => {
    const m = new RegExp(
      `const ${t.rustWorst}: Rgb = Rgb\\((\\d+), (\\d+), (\\d+)\\)`,
    ).exec(appearanceRs);
    expect(m, `falta ${t.rustWorst}`).not.toBeNull();
    const rust = m!.slice(1).map(Number) as Rgb;
    const worst = backgrounds
      .map(([, bg]) => bg)
      .reduce((a, b) =>
        t.name === "claro"
          ? luminance(b) < luminance(a)
            ? b
            : a
          : luminance(b) > luminance(a)
            ? b
            : a,
      );
    for (let i = 0; i < 3; i++)
      expect(Math.abs(rust[i] - worst[i])).toBeLessThan(2);
  });
});
