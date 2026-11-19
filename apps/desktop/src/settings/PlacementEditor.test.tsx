// @vitest-environment jsdom
//
// «Colocar su ventana»: «No colocar» es una casilla más, la pantalla dice su nombre real y
// «Si ya está abierta, usar esa» funciona también sin colocarla.

import "@testing-library/jest-dom/vitest";
import { cleanup, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { PlacementValue } from "./PlacementEditor";

const ok = <T,>(data: T) => Promise.resolve({ status: "ok" as const, data });
const bounds = { left: 0, top: 0, right: 1920, bottom: 1080 };
const commands = {
  listMonitors: vi.fn(() =>
    ok([
      { name: "DELL U2720Q", number: 1, bounds, work: bounds, primary: true },
    ]),
  ),
  identifyMonitors: vi.fn(() => ok(null)),
};
vi.mock("../bindings", () => ({ commands }));

const { PlacementEditor } = await import("./PlacementEditor");

afterEach(cleanup);

function setup(value: PlacementValue | null) {
  const onChange = vi.fn();
  render(<PlacementEditor value={value} onChange={onChange} />);
  return { onChange, user: userEvent.setup() };
}

describe("PlacementEditor", () => {
  it("sin colocar: «No colocar» marcado, sin pantalla, y la casilla de reutilizar a la vista", () => {
    setup(null);
    expect(
      screen.getByRole("radio", { name: "No colocar: donde la ponga Windows" }),
    ).toBeChecked();
    expect(screen.queryByRole("combobox")).not.toBeInTheDocument();
    expect(
      screen.getByRole("checkbox", {
        name: "Si ya está abierta, usar esa en vez de abrir otra",
      }),
    ).not.toBeChecked();
  });

  it("«usar la ya abierta» sin colocar guarda una colocación sin zona", async () => {
    const { onChange, user } = setup(null);
    await user.click(screen.getByRole("checkbox"));
    expect(onChange).toHaveBeenCalledWith({
      zone: null,
      monitor: { kind: "primary" },
      reuseWindow: true,
    });
  });

  it("sin zona y sin reutilizar no queda nada que guardar", async () => {
    const { onChange, user } = setup({
      zone: null,
      monitor: { kind: "primary" },
      reuseWindow: true,
    });
    await user.click(screen.getByRole("checkbox"));
    expect(onChange).toHaveBeenCalledWith(null);
  });

  it("con una posición elegida sale plegado: una línea con lo elegido y «Cambiar»", async () => {
    const { user } = setup({
      zone: "rightHalf",
      monitor: { kind: "primary" },
      reuseWindow: false,
    });
    expect(
      await screen.findByText("Posición: Mitad derecha · Pantalla 1"),
    ).toBeInTheDocument();
    expect(screen.queryByRole("radiogroup")).not.toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: /^Cambiar/ }));
    expect(screen.getByRole("radiogroup")).toBeInTheDocument();
    expect(
      screen.getByText("Posición seleccionada: Mitad derecha"),
    ).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Identificar" }));
    expect(commands.identifyMonitors).toHaveBeenCalled();
  });

  it("con una zona, la pantalla dice su número y su modelo", async () => {
    const { onChange, user } = setup({
      zone: "leftHalf",
      monitor: { kind: "primary" },
      reuseWindow: true,
    });
    await user.click(screen.getByRole("button", { name: /^Cambiar/ }));
    const select = screen.getByRole("combobox", { name: "Pantalla" });
    expect(
      await screen.findByRole("option", {
        name: "Pantalla 1 — DELL U2720Q (principal)",
      }),
    ).toBeInTheDocument();
    expect(select).toHaveDisplayValue("Pantalla 1 — DELL U2720Q (principal)");
    // Pasar a «No colocar» conserva «usar la ya abierta».
    await user.click(
      screen.getByRole("radio", { name: "No colocar: donde la ponga Windows" }),
    );
    expect(onChange).toHaveBeenCalledWith({
      zone: null,
      monitor: { kind: "primary" },
      reuseWindow: true,
    });
  });
});
