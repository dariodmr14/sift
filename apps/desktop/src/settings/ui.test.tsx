// @vitest-environment jsdom
//
// Controles compartidos de Ajustes: lo que oye un lector de pantalla.

import "@testing-library/jest-dom/vitest";
import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import { Field, TextInput } from "./ui";

afterEach(cleanup);

describe("Field", () => {
  it("names the control with its label only and describes it with the hint", () => {
    render(
      <Field label="Programa" hint="Su nombre o su ruta completa.">
        <TextInput />
      </Field>,
    );
    const box = screen.getByRole("textbox", { name: "Programa" });
    expect(box).toHaveAccessibleDescription("Su nombre o su ruta completa.");
  });

  it("works without a hint", () => {
    render(
      <Field label="Nombre">
        <TextInput />
      </Field>,
    );
    const box = screen.getByRole("textbox", { name: "Nombre" });
    expect(box).not.toHaveAttribute("aria-describedby");
  });
});
