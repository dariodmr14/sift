// @vitest-environment jsdom
//
// La bienvenida de verdad: los tres pasos, el permiso para descargar el modelo y el final.
// Rust se sustituye por un falso (`vi.mock("../bindings")`).

import "@testing-library/jest-dom/vitest";
import { act, cleanup, render, screen, waitFor } from "@testing-library/react";
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
afterEach(() => {
  cleanup();
  vi.useRealTimers();
});

describe("Welcome", () => {
  it("enseña los tres pasos y el atajo real", async () => {
    render(<Welcome mode="first-run" />);
    expect(
      await screen.findByRole("heading", {
        name: "Te damos la bienvenida a Sift",
      }),
    ).toBeInTheDocument();
    expect(screen.getByText("El atajo")).toBeInTheDocument();
    expect(screen.getByText("Dónde buscar")).toBeInTheDocument();
    expect(screen.getByText("Búsqueda inteligente")).toBeInTheDocument();
    await waitFor(() =>
      expect(screen.getByLabelText("Alt+Space")).toBeInTheDocument(),
    );
  });

  it("«Descargar» pide el permiso; «Ahora no» no", async () => {
    const user = userEvent.setup();
    render(<Welcome mode="first-run" />);
    await user.click(
      await screen.findByRole("button", { name: "Descargar (120 MB)" }),
    );
    expect(commands.allowModelDownload).toHaveBeenCalledTimes(1);

    cleanup();
    vi.clearAllMocks();
    render(<Welcome mode="first-run" />);
    await user.click(await screen.findByRole("button", { name: "Ahora no" }));
    expect(commands.allowModelDownload).not.toHaveBeenCalled();
    expect(
      screen.getByText(/Puedes activarla cuando quieras/),
    ).toBeInTheDocument();
  });

  it("una carpeta ya añadida no tiene botón", async () => {
    commands.getWelcome.mockImplementation(() => ok(welcome(true)));
    render(<Welcome mode="first-run" />);
    expect(await screen.findByText("Documentos añadida")).toBeInTheDocument();
    expect(
      screen.queryByRole("button", { name: "Añadir Documentos" }),
    ).not.toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: "Añadir Escritorio" }),
    ).toBeInTheDocument();
  });

  it("«Empezar» enseña «Pulsa… para empezar» y abre la barra", async () => {
    vi.useFakeTimers({ shouldAdvanceTime: true });
    const user = userEvent.setup({ advanceTimers: vi.advanceTimersByTime });
    render(<Welcome mode="first-run" />);
    await screen.findByLabelText("Alt+Space");
    await user.click(screen.getByRole("button", { name: "Empezar" }));
    const final = screen.getByRole("status");
    expect(final).toHaveTextContent("Pulsa");
    expect(final).toHaveTextContent("para empezar");
    expect(screen.getByLabelText("Alt+Space")).toBeInTheDocument();
    expect(commands.finishWelcome).not.toHaveBeenCalled();
    await act(async () => {
      await vi.advanceTimersByTimeAsync(1600);
    });
    expect(commands.finishWelcome).toHaveBeenCalledWith(true);
  });
});
