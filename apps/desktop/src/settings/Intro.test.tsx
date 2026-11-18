// @vitest-environment jsdom
//
// La introducción guiada: cinco pasos, Atrás / Siguiente / Saltar, el atajo detectado de
// verdad (Rust avisa cuando se abre la barra), una búsqueda real y el primer Espacio.
// Rust se sustituye por un falso (`vi.mock("../bindings")`).

import "@testing-library/jest-dom/vitest";
import { act, cleanup, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

// jsdom no maqueta nada.
Element.prototype.scrollIntoView = () => {};

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

const welcome = {
  done: false,
  hotkey: "Alt+Space",
  suggested: [
    { kind: "Documents", label: "Documentos", path: "C:\\Docs", added: false },
  ],
};

const hit = {
  fileId: 3,
  chunkId: 1,
  page: null,
  path: "C:\\Docs\\contrato.pdf",
  name: "contrato.pdf",
  ext: "pdf",
  nameSegments: [{ text: "contrato.pdf", hit: false }],
  snippet: [
    { text: "la clave ", hit: false },
    { text: "primaria", hit: true },
  ],
  score: 1,
  boost: 1,
  byMeaning: false,
};

const commands = {
  getWelcome: vi.fn(() => ok(welcome)),
  indexStatus: vi.fn(() => Promise.resolve(status)),
  listRoots: vi.fn(() =>
    ok([
      {
        id: 1,
        path: "C:\\Docs",
        files: 12,
        errors: 0,
        lastIndexedAt: null,
        missing: false,
      },
    ]),
  ),
  addSuggestedFolder: vi.fn(() => ok(null)),
  addRoot: vi.fn(() => ok(null)),
  allowModelDownload: vi.fn(() => ok(null)),
  finishWelcome: vi.fn(() => ok(null)),
  searchFiles: vi.fn((_q: string, seq: number) =>
    ok({ seq, results: { hits: [hit], tookMs: 1 } }),
  ),
  openFile: vi.fn(() => ok(null)),
  createSpace: vi.fn((draft: { name: string }) => ok({ id: 7, ...draft })),
};

// El aviso de «la barra se ha abierto»: el test lo dispara a mano.
let launcherShown: (() => void) | null = null;
const listen = () => Promise.resolve(() => {});
vi.mock("../bindings", () => ({
  commands,
  events: {
    indexProgress: { listen },
    launcherShown: {
      listen: (cb: () => void) => {
        launcherShown = cb;
        return Promise.resolve(() => {});
      },
    },
  },
}));

const { Intro } = await import("./Intro");

beforeEach(() => {
  vi.clearAllMocks();
  launcherShown = null;
});
afterEach(() => {
  cleanup();
  vi.useRealTimers();
});

const title = () => screen.getByRole("heading", { level: 1 });
const next = (user: ReturnType<typeof userEvent.setup>) =>
  user.click(screen.getByRole("button", { name: "Siguiente" }));

describe("Intro", () => {
  it("cinco pasos con su progreso; Atrás y Siguiente", async () => {
    const user = userEvent.setup();
    render(<Intro mode="first-run" onClose={() => {}} />);
    expect(title()).toHaveTextContent("Te damos la bienvenida a Sift");
    expect(screen.getByRole("progressbar")).toHaveAttribute(
      "aria-valuenow",
      "1",
    );
    expect(
      screen.queryByRole("button", { name: "Atrás" }),
    ).not.toBeInTheDocument();
    for (const name of [
      "Dónde buscar",
      "Prueba a buscar",
      "Búsqueda inteligente",
      "Tu primer Espacio",
    ]) {
      await next(user);
      expect(title()).toHaveTextContent(name);
      expect(title()).toHaveFocus();
    }
    expect(screen.getByRole("progressbar")).toHaveAttribute(
      "aria-valuenow",
      "5",
    );
    expect(screen.getByRole("button", { name: "Empezar" })).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Atrás" }));
    expect(title()).toHaveTextContent("Búsqueda inteligente");
  });

  it("detecta el atajo cuando Rust avisa de que la barra se ha abierto", async () => {
    render(<Intro mode="first-run" onClose={() => {}} />);
    expect(await screen.findByLabelText("Alt+Espacio")).toBeInTheDocument();
    expect(screen.queryByText(/Eso es/)).not.toBeInTheDocument();
    await waitFor(() => expect(launcherShown).not.toBeNull());
    act(() => launcherShown?.());
    expect(screen.getByText(/Eso es/)).toBeInTheDocument();
  });

  it("la búsqueda de prueba enseña resultados reales y los abre", async () => {
    const user = userEvent.setup();
    render(<Intro mode="first-run" onClose={() => {}} />);
    await next(user);
    await next(user);
    await user.type(screen.getByRole("textbox", { name: "Buscar" }), "clave");
    await waitFor(() =>
      expect(commands.searchFiles).toHaveBeenLastCalledWith(
        "clave",
        expect.any(Number),
        null,
      ),
    );
    const result = await screen.findByRole("button", { name: /contrato\.pdf/ });
    expect(result).toHaveTextContent("primaria");
    await user.click(result);
    expect(commands.openFile).toHaveBeenCalledWith(
      "C:\\Docs\\contrato.pdf",
      null,
    );
  });

  it("«Descargar» pide el permiso; «Ahora no» no", async () => {
    const user = userEvent.setup();
    render(<Intro mode="first-run" onClose={() => {}} />);
    for (let i = 0; i < 3; i++) await next(user);
    await user.click(screen.getByRole("button", { name: "Ahora no" }));
    expect(commands.allowModelDownload).not.toHaveBeenCalled();
    expect(screen.getByText(/Puedes activarla/)).toBeInTheDocument();
  });

  it("crea el primer Espacio con las carpetas marcadas y la web", async () => {
    const user = userEvent.setup();
    render(<Intro mode="first-run" onClose={() => {}} />);
    for (let i = 0; i < 4; i++) await next(user);
    const create = screen.getByRole("button", { name: "Crear el Espacio" });
    expect(create).toBeDisabled();
    await user.type(screen.getByRole("textbox", { name: "Nombre" }), "Trabajo");
    expect(await screen.findByRole("checkbox", { name: "Docs" })).toBeChecked();
    await user.type(
      screen.getByRole("textbox", { name: /Una web/ }),
      "example.com",
    );
    await user.click(create);
    expect(commands.createSpace).toHaveBeenCalledWith({
      name: "Trabajo",
      aliases: [],
      icon: { kind: "auto" },
      actions: [
        { kind: "openFolder", path: "C:\\Docs" },
        { kind: "openUrl", url: "https://example.com" },
      ],
    });
    expect(await screen.findByText("«Trabajo» creado.")).toBeInTheDocument();
  });

  it("«Saltar» no pregunta: la primera vez la da por vista y vuelve a Ajustes", async () => {
    const user = userEvent.setup();
    const onClose = vi.fn();
    render(<Intro mode="first-run" onClose={onClose} />);
    await user.click(
      screen.getByRole("button", { name: "Saltar la introducción" }),
    );
    expect(commands.finishWelcome).toHaveBeenCalledWith(false);
    await waitFor(() => expect(onClose).toHaveBeenCalled());
  });

  it("repetida desde Ajustes: Terminar vuelve sin tocar nada", async () => {
    const user = userEvent.setup();
    const onClose = vi.fn();
    render(<Intro mode="replay" onClose={onClose} />);
    expect(title()).toHaveTextContent("Tu atajo");
    for (let i = 0; i < 4; i++) await next(user);
    await user.click(screen.getByRole("button", { name: "Terminar" }));
    expect(onClose).toHaveBeenCalled();
    expect(commands.finishWelcome).not.toHaveBeenCalled();
  });

  it("«Empezar» enseña «Pulsa… para empezar» y abre la barra", async () => {
    vi.useFakeTimers({ shouldAdvanceTime: true });
    const user = userEvent.setup({ advanceTimers: vi.advanceTimersByTime });
    render(<Intro mode="first-run" onClose={() => {}} />);
    for (let i = 0; i < 4; i++) await next(user);
    await user.click(screen.getByRole("button", { name: "Empezar" }));
    const final = screen.getByRole("status");
    expect(final).toHaveTextContent("Pulsa");
    expect(final).toHaveTextContent("para empezar");
    expect(commands.finishWelcome).not.toHaveBeenCalled();
    await act(async () => {
      await vi.advanceTimersByTimeAsync(1600);
    });
    expect(commands.finishWelcome).toHaveBeenCalledWith(true);
  });
});
