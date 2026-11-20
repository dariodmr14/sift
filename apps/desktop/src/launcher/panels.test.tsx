// @vitest-environment jsdom
//
// El panel «Abriendo…»: lo que está en marcha lleva el tamiz (el indicador de carga de Sift),
// también las ventanas que aún se colocan; lo ya hecho, su ✓ y nada más.

import "@testing-library/jest-dom/vitest";
import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

vi.mock("../bindings", () => ({ commands: {} }));
const { RunningPanel } = await import("./panels");

afterEach(cleanup);

const loaders = (c: HTMLElement) => c.querySelectorAll(".sift-loader").length;

describe("RunningPanel", () => {
  it("el paso en marcha lleva el tamiz", () => {
    const { container } = render(
      <RunningPanel
        title="Web de la tienda"
        progress={{ step: 1, total: 2, label: "Abrir VS Code", waited: null }}
        steps={[{ step: 1, label: "Abrir VS Code" }]}
        cancelling={false}
      />,
    );
    expect(loaders(container)).toBe(1);
  });

  it("las ventanas que se colocan y el pie «Colocando las ventanas…» llevan el tamiz", () => {
    const { container } = render(
      <RunningPanel
        title="Web de la tienda"
        progress={{
          step: 2,
          total: 2,
          label: "Abrir contrato.pdf",
          waited: null,
        }}
        steps={[
          { step: 1, label: "Abrir VS Code", placement: "placing" },
          { step: 2, label: "Abrir contrato.pdf", placement: "placed" },
        ]}
        cancelling={false}
      />,
    );
    expect(
      screen.getByText("VS Code abierto · colocando…"),
    ).toBeInTheDocument();
    expect(screen.getByText("Colocando las ventanas…")).toBeInTheDocument();
    expect(loaders(container)).toBe(2);
  });

  it("con todo colocado, ningún indicador", () => {
    const { container } = render(
      <RunningPanel
        title="Web de la tienda"
        progress={{
          step: 2,
          total: 2,
          label: "Abrir contrato.pdf",
          waited: null,
        }}
        steps={[
          { step: 1, label: "Abrir VS Code", placement: "placed" },
          { step: 2, label: "Abrir contrato.pdf", placement: "placed" },
        ]}
        cancelling={false}
      />,
    );
    expect(loaders(container)).toBe(0);
  });
});
