import { describe, expect, it } from "vitest";
import { shortcutLabel } from "./shortcut";

describe("atajos en español", () => {
  it("traduce las teclas con nombre en inglés", () => {
    expect(shortcutLabel("Alt+Space")).toBe("Alt+Espacio");
    expect(shortcutLabel("Ctrl+Shift+K")).toBe("Ctrl+Mayús+K");
    expect(shortcutLabel("Ctrl+Alt+Shift+Win+Enter")).toBe(
      "Ctrl+Alt+Mayús+Win+Intro",
    );
    expect(shortcutLabel("Alt+Backquote")).toBe("Alt+º");
  });

  it("deja igual lo que ya está bien y las teclas sueltas", () => {
    expect(shortcutLabel("Win+F2")).toBe("Win+F2");
    expect(shortcutLabel("Alt+Espacio")).toBe("Alt+Espacio");
    expect(shortcutLabel("Ctrl+,")).toBe("Ctrl+,");
    expect(shortcutLabel("Ctrl++")).toBe("Ctrl++");
    expect(shortcutLabel("+")).toBe("+");
    expect(shortcutLabel("Esc")).toBe("Esc");
  });

  it("«enter» en minúscula se queda para que Keycap pinte su glifo", () => {
    expect(shortcutLabel("Ctrl+enter")).toBe("Ctrl+enter");
  });
});
