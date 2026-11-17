// Atajos de teclado como teclas pequeñas, como los menús de Windows 11. Solo en el pie de
// la barra y en la fila seleccionada: en todas las filas serían ruido.

import { Glyph } from "./Glyph";

/** Una tecla. `enter` pinta el glifo de Intro de Windows. */
export function Keycap({ k }: { k: string }) {
  return (
    <kbd className="sift-key">
      {k === "enter" ? <Glyph name="enter" className="text-xs" /> : k}
    </kbd>
  );
}

/** Un atajo («Ctrl+Shift+C», «Ctrl+enter») como teclas seguidas, sin el «+». */
export function Shortcut({ keys }: { keys: string }) {
  const parts = keys === "+" ? ["+"] : keys.split(/\+(?!$)/);
  return (
    <span className="inline-flex items-center gap-0.5" aria-label={keys}>
      {parts.map((k, i) => (
        <Keycap key={i} k={k} />
      ))}
    </span>
  );
}
