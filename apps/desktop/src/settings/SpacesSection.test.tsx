// @vitest-environment jsdom
//
// La lista de Espacios: pintarlos, crear uno nuevo, reordenar con Alt+↑/↓ e importar/exportar.
// Rust se sustituye por un falso (`vi.mock("../bindings")`).

import "@testing-library/jest-dom/vitest";
import {
  cleanup,
  render,
  screen,
  waitFor,
  within,
} from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

const ok = <T,>(data: T) => Promise.resolve({ status: "ok" as const, data });

const view = (id: number, name: string) => ({
  space: {
    id,
    name,
    aliases: [],
    icon: { kind: "none" },
    actions: [],
  },
  autoIcon: null,
  boost: null,
});

const commands = {
  listMonitors: vi.fn(() => ok([] as unknown[])),
  listSpaces: vi.fn(() =>
    ok([view(1, "Clase"), view(2, "Trabajo"), view(3, "Ocio")]),
  ),
  reorderSpaces: vi.fn((_ids: number[]) => ok(null)),
  exportSpaces: vi.fn(() => ok(null as string | null)),
  importSpaces: vi.fn(() => ok(null as number | null)),
  getIcons: vi.fn((_k: string[]) => ok([] as (string | null)[])),
};
const events = {
  newSpaceRequested: { listen: vi.fn(() => Promise.resolve(() => {})) },
};
vi.mock("../bindings", () => ({ commands, events }));

// jsdom no trae `matchMedia` (lo usa el arrastre).
vi.stubGlobal("matchMedia", (query: string) => ({
  matches: false,
  media: query,
  addEventListener: () => {},
  removeEventListener: () => {},
}));

const { SpacesSection } = await import("./SpacesSection");

const list = () => screen.getByRole("list", { name: "Espacios" });
const rowNames = () =>
  within(list())
    .getAllByRole("button")
    .map((b) => b.textContent);

beforeEach(() => vi.clearAllMocks());
afterEach(cleanup);

describe("SpacesSection", () => {
  it("lists the spaces from listSpaces and opens the first one", async () => {
    render(<SpacesSection />);
    expect(await screen.findByText("Trabajo")).toBeInTheDocument();
    expect(rowNames()).toEqual(["Clase", "Trabajo", "Ocio"]);
    await waitFor(() =>
      expect(screen.getByLabelText("Nombre")).toHaveValue("Clase"),
    );
  });

  it("«Nuevo Espacio» opens an empty editor", async () => {
    const user = userEvent.setup();
    render(<SpacesSection />);
    await screen.findByText("Trabajo");
    await user.click(screen.getByRole("button", { name: /Nuevo Espacio/ }));
    expect(screen.getByLabelText("Nombre")).toHaveValue("");
    expect(
      screen.getByRole("button", { name: "Crear Espacio" }),
    ).toBeInTheDocument();
  });

  it("shows the welcome message when there are no spaces", async () => {
    commands.listSpaces.mockImplementationOnce(() => ok([]));
    render(<SpacesSection />);
    expect(await screen.findByText("Crea tu primer Espacio")).toBeVisible();
    expect(screen.getByText("Aún no hay Espacios")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /Exportar/ })).toBeDisabled();
  });

  it("Alt+↓ moves a space down and saves the new order", async () => {
    const user = userEvent.setup();
    render(<SpacesSection />);
    await screen.findByText("Trabajo");
    within(list()).getByRole("button", { name: "Clase" }).focus();
    await user.keyboard("{Alt>}{ArrowDown}{/Alt}");
    expect(commands.reorderSpaces).toHaveBeenCalledWith([2, 1, 3]);
    expect(rowNames()).toEqual(["Trabajo", "Clase", "Ocio"]);
  });

  it("Alt+↑ moves a space up and saves the new order", async () => {
    const user = userEvent.setup();
    render(<SpacesSection />);
    await screen.findByText("Trabajo");
    within(list()).getByRole("button", { name: "Ocio" }).focus();
    await user.keyboard("{Alt>}{ArrowUp}{/Alt}");
    expect(commands.reorderSpaces).toHaveBeenCalledWith([1, 3, 2]);
    expect(rowNames()).toEqual(["Clase", "Ocio", "Trabajo"]);
  });

  it("Alt+↑ on the first space changes nothing on screen", async () => {
    const user = userEvent.setup();
    render(<SpacesSection />);
    await screen.findByText("Trabajo");
    within(list()).getByRole("button", { name: "Clase" }).focus();
    await user.keyboard("{Alt>}{ArrowUp}{/Alt}");
    expect(rowNames()).toEqual(["Clase", "Trabajo", "Ocio"]);
  });

  it("«Exportar…» calls exportSpaces and says where it went", async () => {
    const user = userEvent.setup();
    commands.exportSpaces.mockImplementationOnce(() => ok("C:\\a\\x.json"));
    render(<SpacesSection />);
    await screen.findByText("Trabajo");
    await user.click(screen.getByRole("button", { name: "Exportar…" }));
    expect(commands.exportSpaces).toHaveBeenCalledTimes(1);
    expect(await screen.findByText("Exportados a C:\\a\\x.json")).toBeVisible();
  });

  it("«Importar…» calls importSpaces, reports the count and reloads", async () => {
    const user = userEvent.setup();
    commands.importSpaces.mockImplementationOnce(() => ok(2));
    render(<SpacesSection />);
    await screen.findByText("Trabajo");
    expect(commands.listSpaces).toHaveBeenCalledTimes(1);
    await user.click(screen.getByRole("button", { name: "Importar…" }));
    expect(commands.importSpaces).toHaveBeenCalledTimes(1);
    expect(await screen.findByText(/2 Espacios importados/)).toBeVisible();
    await waitFor(() => expect(commands.listSpaces).toHaveBeenCalledTimes(2));
  });

  it("an import that is cancelled neither reports nor reloads", async () => {
    const user = userEvent.setup();
    render(<SpacesSection />);
    await screen.findByText("Trabajo");
    await user.click(screen.getByRole("button", { name: "Importar…" }));
    expect(commands.importSpaces).toHaveBeenCalledTimes(1);
    expect(commands.listSpaces).toHaveBeenCalledTimes(1);
    expect(screen.queryByText(/importado/)).not.toBeInTheDocument();
  });

  it("an import error is shown", async () => {
    const user = userEvent.setup();
    commands.importSpaces.mockImplementationOnce(
      () =>
        Promise.resolve({
          status: "error" as const,
          error: { message: "Archivo no válido" },
        }) as never,
    );
    render(<SpacesSection />);
    await screen.findByText("Trabajo");
    await user.click(screen.getByRole("button", { name: "Importar…" }));
    expect(
      await screen.findByText("No se importó nada: Archivo no válido"),
    ).toBeVisible();
  });
});
