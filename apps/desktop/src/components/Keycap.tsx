// Atajos de teclado como teclas pequeñas, como los menús de Windows 11. Solo en el pie de
// la barra y en la fila seleccionada: en todas las filas serían ruido.

import { shortcutLabel } from "../shortcut";
import { Glyph } from "./Glyph";

/** Una tecla. `enter` pinta el glifo de Intro de Windows. */
export function Keycap({ k }: { k: string }) {
  return (
    <kbd className="sift-key">
      {k === "enter" ? <Glyph name="enter" className="text-xs" /> : k}
    </kbd>
  );
}

/**
 * Un atajo («Ctrl+Shift+C», «Ctrl+enter») como teclas seguidas, sin el «+» y con los
 * nombres del teclado español («Ctrl+Mayús+C»).
 */
export function Shortcut({ keys }: { keys: string }) {
  const label = shortcutLabel(keys);
  const parts = label === "+" ? ["+"] : label.split(/\+(?!$)/);
  return (
    <span className="inline-flex items-center gap-0.5" aria-label={label}>
      {parts.map((k, i) => (
        <Keycap key={i} k={k} />
      ))}
    </span>
  );
}
