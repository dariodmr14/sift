// @vitest-environment jsdom
// La firma («la marca de lo que encaja») y las teclas de los atajos.

import { render } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { Shortcut } from "../components/Keycap";
import { Highlighted } from "./panels";

const segments = [
  { text: "Normalizar ", hit: false },
  { text: "tablas", hit: true },
  { text: " evita la redundancia", hit: false },
];

describe("la marca de lo que encaja", () => {
  it("en una búsqueda exacta marca solo las palabras encontradas", () => {
    const { container } = render(<Highlighted segments={segments} />);
    const marks = [...container.querySelectorAll("mark.sift-match")];
    expect(marks.map((m) => m.textContent)).toEqual(["tablas"]);
  });

  it("por significado marca el pasaje entero, con la misma marca", () => {
    const { container } = render(
      <Highlighted
        segments={segments.map((s) => ({ ...s, hit: false }))}
        whole
      />,
    );
    const marks = [...container.querySelectorAll("mark.sift-match")];
    expect(marks.map((m) => m.textContent)).toEqual([
      "Normalizar tablas evita la redundancia",
    ]);
  });
});

describe("atajos como teclas", () => {
  const keys = (shortcut: string) => {
    const { container } = render(<Shortcut keys={shortcut} />);
    return [...container.querySelectorAll("kbd")].map((k) => k.textContent);
  };

  it("separa las teclas por el «+»", () => {
    expect(keys("Ctrl+Shift+C")).toEqual(["Ctrl", "Shift", "C"]);
  });

  it("la tecla «,» y la del «+» también", () => {
    expect(keys("Ctrl+,")).toEqual(["Ctrl", ","]);
    expect(keys("Ctrl++")).toEqual(["Ctrl", "+"]);
  });

  it("Intro es el glifo de Windows, no texto", () => {
    expect(keys("enter")).toEqual([""]);
  });
});
