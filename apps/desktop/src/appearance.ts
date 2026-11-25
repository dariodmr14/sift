import { commands, events } from "./bindings";

/**
 * Aplica el tema y el acento de Windows (los lee Rust) a la página: `data-theme` y las
 * variables `--accent`/`--accent-fg`/`--accent-ink` en <html>. El resto de tokens cambian solos con
 * `data-theme` (src/index.css).
 */
export async function applyAppearance(): Promise<void> {
  const a = await commands.getAppearance();
  const root = document.documentElement;
  root.dataset.theme = a.theme;
  root.style.setProperty("--accent", a.accent);
  root.style.setProperty("--accent-fg", a.accentText);
  root.style.setProperty("--accent-ink", a.accentInk);
}

/**
 * Pregunta a Rust qué material de Windows tiene esta ventana y lo marca en <html>
 * (`data-glass`): con material, el fondo es translúcido; sin él, opaco.
 */
export async function applyWindowMaterial(): Promise<void> {
  const material = await commands.getWindowMaterial();
  document.documentElement.dataset.glass = String(material !== "none");
}

/**
 * Repinta la ventana cuando cambian el tema o el vidrio (Ajustes → General → Apariencia,
 * o Windows deja de poder pintar el vidrio). Devuelve la promesa para dejar de escuchar.
 */
export function followAppearanceChanges(): Promise<() => void> {
  return events.appearanceChanged.listen(() => {
    void applyAppearance();
    void applyWindowMaterial();
  });
}

/** ¿Tiene Windows pedido «reducir movimiento»? */
export function prefersReducedMotion(): boolean {
  return window.matchMedia("(prefers-reduced-motion: reduce)").matches;
}
