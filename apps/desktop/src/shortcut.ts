// Los atajos se guardan con los nombres en inglés de Windows («Alt+Space», «Ctrl+Shift+K»),
// pero en la interfaz se ven como en un teclado español («Alt+Espacio», «Ctrl+Mayús+K»).
// Es el único sitio que traduce: lo usan las teclas dibujadas (`Shortcut`), Ajustes y la
// introducción. Rust entiende las dos formas, así que lo que escriba la persona vale tal cual.

const ES: Record<string, string> = {
  space: "Espacio",
  shift: "Mayús",
  enter: "Intro",
  return: "Intro",
  backquote: "º",
  control: "Ctrl",
  ctrl: "Ctrl",
  super: "Win",
  meta: "Win",
};

/** Una tecla, en español. `enter` en minúscula se queda: `Keycap` pinta su glifo. */
function keyLabel(key: string): string {
  if (key === "enter") return key;
  return ES[key.toLowerCase()] ?? key;
}

/** «Alt+Space» → «Alt+Espacio»; «Ctrl+Shift+K» → «Ctrl+Mayús+K». */
export function shortcutLabel(shortcut: string): string {
  if (shortcut === "+") return shortcut;
  return shortcut
    .split(/\+(?!$)/)
    .map((k) => keyLabel(k.trim()))
    .join("+");
}
