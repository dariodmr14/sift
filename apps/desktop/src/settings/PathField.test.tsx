// @vitest-environment jsdom
//
// El campo de ruta con sugerencias: pide al escribir, se maneja con teclado y no molesta
// cuando ya escribes una ruta completa.

import "@testing-library/jest-dom/vitest";
import { cleanup, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { useState } from "react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

const ok = <T,>(data: T) => Promise.resolve({ status: "ok" as const, data });

const commands = {
  suggestPaths: vi.fn((_t: string, _k: string) =>
    ok(["C:\\Docs\\tema4", "C:\\Docs\\tema5"]),
  ),
  pickPath: vi.fn((_k: string) => ok(null as string | null)),
};
vi.mock("../bindings", () => ({ commands }));

const { PathField } = await import("./PathField");

/** Un padre que guarda el valor, como hace el editor de acciones. */
function setup() {
  const onChange = vi.fn();
  function Host() {
    const [v, setV] = useState("");
    return (
      <PathField
        label="Ruta"
        kind="folder"
        placeholder="…"
        value={v}
        onChange={(p) => {
          onChange(p);
          setV(p);
        }}
      />
    );
  }
  render(<Host />);
  return onChange;
}

/** Deja pasar el debounce (120 ms) para comprobar que NO se pidió nada. */
const settle = () => new Promise((r) => setTimeout(r, 250));

beforeEach(() => vi.clearAllMocks());
afterEach(cleanup);

describe("PathField", () => {
  it("asks for suggestions after two letters and lists them", async () => {
    const user = userEvent.setup();
    setup();
    await user.type(screen.getByRole("combobox", { name: "Ruta" }), "te");
    expect(await screen.findAllByRole("option")).toHaveLength(2);
    expect(commands.suggestPaths).toHaveBeenLastCalledWith("te", "folder");
  });

  it("one letter is not enough", async () => {
    const user = userEvent.setup();
    setup();
    await user.type(screen.getByRole("combobox"), "t");
    await settle();
    expect(commands.suggestPaths).not.toHaveBeenCalled();
  });

  it("a full path does not ask for suggestions", async () => {
    const user = userEvent.setup();
    setup();
    await user.type(screen.getByRole("combobox"), "C:\\Users\\x");
    await settle();
    expect(commands.suggestPaths).not.toHaveBeenCalled();
    expect(screen.queryByRole("listbox")).not.toBeInTheDocument();
  });

  it("↓ and Enter choose a suggestion", async () => {
    const user = userEvent.setup();
    const onChange = setup();
    await user.type(screen.getByRole("combobox"), "te");
    await screen.findAllByRole("option");
    await user.keyboard("{ArrowDown}{ArrowDown}{Enter}");
    expect(onChange).toHaveBeenLastCalledWith("C:\\Docs\\tema5");
    expect(screen.queryByRole("listbox")).not.toBeInTheDocument();
    expect(screen.getByRole("combobox")).toHaveValue("C:\\Docs\\tema5");
  });

  it("Enter without a highlighted option chooses nothing", async () => {
    const user = userEvent.setup();
    const onChange = setup();
    await user.type(screen.getByRole("combobox"), "te");
    await screen.findAllByRole("option");
    onChange.mockClear();
    await user.keyboard("{Enter}");
    expect(onChange).not.toHaveBeenCalled();
  });

  it("clicking a suggestion chooses it", async () => {
    const user = userEvent.setup();
    const onChange = setup();
    await user.type(screen.getByRole("combobox"), "te");
    await user.click((await screen.findAllByRole("option"))[0]);
    expect(onChange).toHaveBeenLastCalledWith("C:\\Docs\\tema4");
  });

  it("Esc closes the list and keeps what you typed", async () => {
    const user = userEvent.setup();
    setup();
    await user.type(screen.getByRole("combobox"), "te");
    await screen.findByRole("listbox");
    await user.keyboard("{Escape}");
    expect(screen.queryByRole("listbox")).not.toBeInTheDocument();
    expect(screen.getByRole("combobox")).toHaveValue("te");
  });

  it("«Elegir…» uses the Windows picker", async () => {
    commands.pickPath.mockImplementationOnce(() => ok("C:\\Elegida"));
    const user = userEvent.setup();
    const onChange = setup();
    await user.click(screen.getByRole("button", { name: "Elegir…" }));
    await waitFor(() =>
      expect(onChange).toHaveBeenLastCalledWith("C:\\Elegida"),
    );
    expect(commands.pickPath).toHaveBeenCalledWith("folder");
  });
});
