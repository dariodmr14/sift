// @vitest-environment jsdom
//
// Tests de la barra de verdad (el componente, no solo su lógica): teclado, filtros,
// abrir y cancelar un Espacio. Rust se sustituye por un falso (`vi.mock("./bindings")`).

import "@testing-library/jest-dom/vitest";
import { act, cleanup, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

// jsdom no maqueta nada: no tiene `scrollIntoView`.
Element.prototype.scrollIntoView = () => {};

const ok = <T,>(data: T) => Promise.resolve({ status: "ok" as const, data });

const space = {
  space: {
    id: 1,
    name: "Trabajo",
    aliases: ["curro"],
    icon: { kind: "none" },
    actions: [],
  },
  autoIcon: null,
  boost: 1,
};

const hit = (name: string, page: number | null = null) => ({
  fileId: name.length,
  chunkId: 1,
  page,
  path: `C:\\Docs\\${name}`,
  name,
  ext: name.split(".").pop(),
  nameSegments: [{ text: name, hit: false }],
  snippet: [],
  score: 1,
  boost: 1,
  byMeaning: false,
});

// Lo que la barra le pide a Rust. Cada test cambia lo que necesite.
const commands = {
  listSpaces: vi.fn(() => ok([space])),
  getSettings: vi.fn(() => Promise.resolve({ iconsHidden: true })),
  recentFiles: vi.fn(() =>
    ok([
      {
        fileId: 9,
        path: "C:\\Docs\\informe.docx",
        name: "informe.docx",
        ext: "docx",
      },
    ]),
  ),
  searchFiles: vi.fn((_q: string, seq: number) =>
    ok({
      seq,
      results: { hits: [hit("contrato.pdf", 37), hit("notas.md")], tookMs: 1 },
    }),
  ),
  getIcons: vi.fn((paths: string[]) => ok(paths.map(() => null))),
  openFile: vi.fn(() => ok(null)),
  revealFile: vi.fn(() => ok(null)),
  runSpace: vi.fn(),
  cancelSpaceRun: vi.fn(() => Promise.resolve()),
  openSettings: vi.fn(() => Promise.resolve()),
  hideLauncher: vi.fn(() => Promise.resolve()),
  launcherPainted: vi.fn(() => Promise.resolve()),
};
const listen = () => Promise.resolve(() => {});
vi.mock("./bindings", () => ({
  commands,
  events: {
    launcherShown: { listen },
    launcherHidden: { listen },
    spaceProgress: { listen },
  },
}));
vi.mock("./appearance", () => ({
  applyAppearance: () => Promise.resolve(),
  applyWindowMaterial: () => Promise.resolve(),
  prefersReducedMotion: () => true,
}));

const { default: App } = await import("./App");

beforeEach(() => {
  vi.clearAllMocks();
});
afterEach(cleanup);

async function renderBar() {
  const user = userEvent.setup();
  render(<App />);
  const box = screen.getByRole<HTMLInputElement>("combobox", {
    name: "Buscar",
  });
  await waitFor(() => expect(commands.listSpaces).toHaveBeenCalled());
  return { user, box };
}

const selectedText = () =>
  screen
    .getAllByRole("option")
    .find((o) => o.getAttribute("aria-selected") === "true")?.textContent;

describe("barra", () => {
  it("vacía: enseña los Espacios y lo abierto hace poco", async () => {
    await renderBar();
    expect(await screen.findByText("Espacios")).toBeInTheDocument();
    expect(screen.getByText("Abierto recientemente")).toBeInTheDocument();
    expect(screen.getByText("informe.docx")).toBeInTheDocument();
  });

  it("anuncia el número de resultados (con retardo) en la zona de estado", async () => {
    const { user, box } = await renderBar();
    await user.type(box, "contrato");
    await waitFor(() =>
      expect(screen.getByRole("status")).toHaveTextContent(/^\d+ resultados?/),
    );
  });

  it("«pdf » se convierte en etiqueta, busca solo PDF y Retroceso la quita", async () => {
    const { user, box } = await renderBar();
    await user.type(box, "pdf contrato");
    expect(screen.getByRole("button", { name: /PDF/ })).toBeInTheDocument();
    expect(box).toHaveValue("contrato");
    await waitFor(() =>
      expect(commands.searchFiles).toHaveBeenLastCalledWith(
        "contrato",
        expect.any(Number),
        "pdf",
      ),
    );
    // Retroceso al principio de la caja: fuera la etiqueta, el texto se queda.
    box.setSelectionRange(0, 0);
    await user.keyboard("{Backspace}");
    expect(
      screen.queryByRole("button", { name: /PDF/ }),
    ).not.toBeInTheDocument();
    expect(box).toHaveValue("contrato");
  });

  it("↓ mueve la selección y Enter abre el PDF en su página", async () => {
    const { user, box } = await renderBar();
    await user.type(box, "contrato");
    expect(
      await screen.findByText("pág. 37 ·", { exact: false }),
    ).toBeInTheDocument();
    await waitFor(() => expect(selectedText()).toContain("contrato.pdf"));
    await user.keyboard("{ArrowDown}");
    expect(selectedText()).toContain("notas.md");
    await user.keyboard("{ArrowUp}{Enter}");
    expect(commands.openFile).toHaveBeenCalledWith(
      "C:\\Docs\\contrato.pdf",
      37,
    );
  });

  it("un Espacio que tarda: panel «en marcha», Esc cancela y luego el resumen", async () => {
    let finish!: (value: unknown) => void;
    commands.runSpace.mockReturnValue(new Promise((r) => (finish = r)));
    const { user, box } = await renderBar();
    await user.type(box, "curro");
    await user.keyboard("{Enter}");
    expect(
      await screen.findByText("Abriendo «Trabajo»", {}, { timeout: 2000 }),
    ).toBeInTheDocument();
    await user.keyboard("{Escape}");
    expect(commands.cancelSpaceRun).toHaveBeenCalled();
    expect(screen.getByText("Cancelando…")).toBeInTheDocument();
    // Rust contesta: se deja que la promesa se resuelva dentro de `act`.
    await act(async () => {
      finish({
        status: "ok",
        data: {
          status: "done",
          outcomes: [
            {
              index: 1,
              label: "Abrir VS Code",
              error: null,
              ran: true,
              note: null,
            },
            {
              index: 2,
              label: "Abrir web",
              error: null,
              ran: false,
              note: null,
            },
          ],
        },
      });
      await Promise.resolve();
    });
    expect(
      await screen.findByText(
        "«Trabajo»: Cancelado: lo que ya se abrió sigue abierto",
      ),
    ).toBeInTheDocument();
    expect(screen.getByText(/no se ejecutó/)).toBeInTheDocument();
  });

  it("si se cancela la confirmación de Windows, vuelve a la lista sin más", async () => {
    commands.runSpace.mockReturnValue(ok({ status: "declined" }));
    const { user, box } = await renderBar();
    await user.type(box, "curro");
    await user.keyboard("{Enter}");
    await waitFor(() => expect(commands.runSpace).toHaveBeenCalledWith(1));
    expect(screen.queryByRole("alert")).not.toBeInTheDocument();
    expect(screen.getByRole("listbox")).toBeInTheDocument();
  });
});
