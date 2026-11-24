// @vitest-environment jsdom
//
// Ajustes → Primeros pasos: los tres apartados, el permiso para descargar el modelo y el
// botón para repetir la introducción.
// Rust se sustituye por un falso (`vi.mock("../bindings")`).

import "@testing-library/jest-dom/vitest";
import { cleanup, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

const ok = <T,>(data: T) => Promise.resolve({ status: "ok" as const, data });

const smart = {
  enabled: true,
  ready: false,
  done: 0,
  total: 0,
  downloadedMb: 0,
  downloadMb: 120,
  awaitingPermission: true,
  note: null,
};

const status = { running: false, phase: "idle", smart };

const welcome = (added: boolean) => ({
  done: false,
  hotkey: "Alt+Space",
  suggested: [
    { kind: "Documents", label: "Documentos", path: "C:\\Docs", added },
    { kind: "Desktop", label: "Escritorio", path: "C:\\Desk", added: false },
  ],
});

const commands = {
  getWelcome: vi.fn(() => ok(welcome(false))),
  indexStatus: vi.fn(() => Promise.resolve(status)),
  addSuggestedFolder: vi.fn(() => ok(null)),
  addRoot: vi.fn(() => ok(null)),
  allowModelDownload: vi.fn(() => ok(null)),
  finishWelcome: vi.fn(() => ok(null)),
};
const listen = () => Promise.resolve(() => {});
vi.mock("../bindings", () => ({
  commands,
  events: { indexProgress: { listen } },
}));

const { Welcome } = await import("./Welcome");

beforeEach(() => {
  vi.clearAllMocks();
  commands.getWelcome.mockImplementation(() => ok(welcome(false)));
});
afterEach(cleanup);

describe("Welcome (Ajustes → Primeros pasos)", () => {
  it("enseña los tres apartados y el atajo real", async () => {
    render(<Welcome />);
    expect(screen.getByText("El atajo")).toBeInTheDocument();
    expect(screen.getByText("Dónde buscar")).toBeInTheDocument();
    expect(screen.getByText("Búsqueda inteligente")).toBeInTheDocument();
    await waitFor(() =>
      expect(screen.getByLabelText("Alt+Espacio")).toBeInTheDocument(),
    );
  });

  it("«Descargar» pide el permiso; «Ahora no» no", async () => {
    const user = userEvent.setup();
    render(<Welcome />);
    await user.click(
      await screen.findByRole("button", { name: "Descargar (135 MB)" }),
    );
    expect(commands.allowModelDownload).toHaveBeenCalledTimes(1);

    cleanup();
    vi.clearAllMocks();
    render(<Welcome />);
    await user.click(await screen.findByRole("button", { name: "Ahora no" }));
    expect(commands.allowModelDownload).not.toHaveBeenCalled();
    expect(
      screen.getByText(/Puedes activarla cuando quieras/),
    ).toBeInTheDocument();
  });

  it("una carpeta ya añadida no tiene botón", async () => {
    commands.getWelcome.mockImplementation(() => ok(welcome(true)));
    render(<Welcome />);
    expect(await screen.findByText("Documentos añadida")).toBeInTheDocument();
    expect(
      screen.queryByRole("button", { name: "Añadir Documentos" }),
    ).not.toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: "Añadir Escritorio" }),
    ).toBeInTheDocument();
  });

  it("«Ver la introducción otra vez» y «Cambiar» llevan a su sitio", async () => {
    const user = userEvent.setup();
    const onReplay = vi.fn();
    const onGoTo = vi.fn();
    render(<Welcome onReplay={onReplay} onGoTo={onGoTo} />);
    await user.click(
      screen.getByRole("button", { name: "Ver la introducción otra vez" }),
    );
    expect(onReplay).toHaveBeenCalled();
    await user.click(screen.getByRole("button", { name: "Cambiar" }));
    expect(onGoTo).toHaveBeenCalledWith("general");
  });
});
