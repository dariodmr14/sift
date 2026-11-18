// @vitest-environment jsdom
//
// El editor de un Espacio: guardar, «Guardado» sin cambios y borrar con confirmación.
// Rust se sustituye por un falso (`vi.mock("../bindings")`).

import "@testing-library/jest-dom/vitest";
import { cleanup, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { Draft } from "./draft";

const ok = <T,>(data: T) => Promise.resolve({ status: "ok" as const, data });

const commands = {
  listMonitors: vi.fn(() => ok([] as unknown[])),
  createSpace: vi.fn((_d: unknown) => ok({ id: 9 })),
  updateSpace: vi.fn((_id: number, _d: unknown) => ok({ id: 7 })),
  deleteSpace: vi.fn((_id: number) => ok(null)),
  duplicateSpace: vi.fn((_id: number) => ok({ id: 8 })),
  getIcons: vi.fn((_k: string[]) => ok([] as (string | null)[])),
};
vi.mock("../bindings", () => ({ commands }));

// jsdom no trae `matchMedia` (lo usa el arrastre de acciones).
vi.stubGlobal("matchMedia", (query: string) => ({
  matches: false,
  media: query,
  addEventListener: () => {},
  removeEventListener: () => {},
}));

const { SpaceEditor } = await import("./SpaceEditor");

const saved = (): Draft => ({
  name: "Clase",
  aliases: ["uni"],
  icon: { kind: "auto" },
  actions: [
    { kind: "openUrl", url: "https://example.com" },
    { kind: "runCommand", program: "git", args: ["status"], cwd: null },
  ],
});

function setup(id: number | null = 7) {
  const props = {
    onSaved: vi.fn(),
    onDeleted: vi.fn(),
    onDuplicated: vi.fn(),
  };
  const draft = saved();
  render(
    <SpaceEditor
      id={id}
      saved={id === null ? null : draft}
      initial={draft}
      {...props}
    />,
  );
  return props;
}

beforeEach(() => vi.clearAllMocks());
afterEach(cleanup);

describe("SpaceEditor", () => {
  it("saves the new name with the actions untouched", async () => {
    const user = userEvent.setup();
    const props = setup();
    const name = screen.getByLabelText("Nombre");
    await user.clear(name);
    await user.type(name, "Trabajo");
    await user.click(screen.getByRole("button", { name: "Guardar" }));
    expect(commands.updateSpace).toHaveBeenCalledTimes(1);
    expect(commands.updateSpace).toHaveBeenCalledWith(7, {
      ...saved(),
      name: "Trabajo",
    });
    expect(props.onSaved).toHaveBeenCalledWith(7);
  });

  it("Ctrl+S also saves", async () => {
    const user = userEvent.setup();
    setup();
    await user.type(screen.getByLabelText("Nombre"), "!");
    await user.keyboard("{Control>}s{/Control}");
    expect(commands.updateSpace).toHaveBeenCalledTimes(1);
  });

  it("«Guardar» is disabled and says «Guardado» without changes", () => {
    setup();
    expect(screen.getByRole("button", { name: "Guardar" })).toBeDisabled();
    expect(screen.getByText("Guardado")).toBeInTheDocument();
  });

  it("editing enables «Guardar» and «Descartar cambios» restores the name", async () => {
    const user = userEvent.setup();
    setup();
    await user.type(screen.getByLabelText("Nombre"), "X");
    expect(screen.getByRole("button", { name: "Guardar" })).toBeEnabled();
    expect(screen.getByText(/Cambios sin guardar/)).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Descartar cambios" }));
    expect(screen.getByLabelText("Nombre")).toHaveValue("Clase");
    expect(screen.getByRole("button", { name: "Guardar" })).toBeDisabled();
  });

  it("a new space is created with «Crear Espacio»", async () => {
    const user = userEvent.setup();
    const props = setup(null);
    await user.type(screen.getByLabelText("Nombre"), "Nuevo");
    await user.click(screen.getByRole("button", { name: "Crear Espacio" }));
    expect(commands.createSpace).toHaveBeenCalledTimes(1);
    expect(props.onSaved).toHaveBeenCalledWith(9);
  });

  it("shows the error Rust returns and does not report it as saved", async () => {
    commands.updateSpace.mockImplementationOnce(
      () =>
        Promise.resolve({
          status: "error" as const,
          error: { message: "El nombre no puede estar vacío." },
        }) as never,
    );
    const user = userEvent.setup();
    const props = setup();
    await user.clear(screen.getByLabelText("Nombre"));
    await user.click(screen.getByRole("button", { name: "Guardar" }));
    expect(
      await screen.findByText("El nombre no puede estar vacío."),
    ).toBeInTheDocument();
    expect(props.onSaved).not.toHaveBeenCalled();
  });

  it("«Borrar» asks first; «No» keeps the space", async () => {
    const user = userEvent.setup();
    const props = setup();
    await user.click(screen.getByRole("button", { name: "Borrar" }));
    expect(screen.getByText("¿Borrar «Clase»?")).toBeInTheDocument();
    expect(commands.deleteSpace).not.toHaveBeenCalled();
    await user.click(screen.getByRole("button", { name: "No" }));
    expect(screen.queryByText("¿Borrar «Clase»?")).not.toBeInTheDocument();
    expect(commands.deleteSpace).not.toHaveBeenCalled();
    expect(props.onDeleted).not.toHaveBeenCalled();
  });

  it("confirming the deletion calls deleteSpace with its id", async () => {
    const user = userEvent.setup();
    const props = setup();
    await user.click(screen.getByRole("button", { name: "Borrar" }));
    await user.click(screen.getByRole("button", { name: "Sí, borrar" }));
    expect(commands.deleteSpace).toHaveBeenCalledWith(7);
    expect(props.onDeleted).toHaveBeenCalledTimes(1);
  });

  it("removing an action only changes the draft until it is saved", async () => {
    const user = userEvent.setup();
    setup();
    const remove = screen.getAllByRole("button", { name: "Quitar acción" });
    await user.click(remove[0]);
    expect(commands.updateSpace).not.toHaveBeenCalled();
    await user.click(screen.getByRole("button", { name: "Guardar" }));
    expect(commands.updateSpace).toHaveBeenCalledWith(7, {
      ...saved(),
      actions: [saved().actions[1]],
    });
  });
});
