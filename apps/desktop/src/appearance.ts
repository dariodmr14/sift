import { commands } from "./bindings";

/**
 * Aplica el tema y el acento de Windows (los lee Rust) a la página: `data-theme` y las
 * variables `--accent`/`--accent-fg` en <html>. El resto de tokens cambian solos con
 * `data-theme` (src/index.css).
 */
export async function applyAppearance(): Promise<void> {
  const a = await commands.getAppearance();
  const root = document.documentElement;
  root.dataset.theme = a.theme;
  root.style.setProperty("--accent", a.accent);
  root.style.setProperty("--accent-fg", a.accentText);
}

/** ¿Tiene Windows pedido «reducir movimiento»? */
export function prefersReducedMotion(): boolean {
  return window.matchMedia("(prefers-reduced-motion: reduce)").matches;
}
