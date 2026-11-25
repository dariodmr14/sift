// @vitest-environment jsdom
//
// Tests de la barra de verdad (el componente, no solo su lógica): teclado, filtros,
// abrir y cancelar un Espacio. Rust se sustituye por un falso (`vi.mock("./bindings")`).

import "@testing-library/jest-dom/vitest";
import {
  act,
  cleanup,
  fireEvent,
  render,
  screen,
  waitFor,
} from "@testing-library/react";
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
    actions: [] as object[],
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
  snippet: [] as { text: string; hit: boolean }[],
  score: 1,
  boost: 1,
  byMeaning: false,
});

const defaultSearch = (_q: string, seq: number) =>
  ok({
    seq,
    results: { hits: [hit("contrato.pdf", 37), hit("notas.md")], tookMs: 1 },
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
  searchFiles: vi.fn(defaultSearch),
  getIcons: vi.fn((paths: string[]) => ok(paths.map(() => null))),
  previewFile: vi.fn((_fileId: number, _chunkId: number, _query: string) =>
    ok({
      text: [
        { text: "la clave ", hit: false },
        { text: "primaria", hit: true },
      ],
      monospace: false,
      page: 37,
    }),
  ),
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
    spacePlacement: { listen },
  },
}));
vi.mock("./appearance", () => ({
  applyAppearance: () => Promise.resolve(),
  applyWindowMaterial: () => Promise.resolve(),
  followAppearanceChanges: () => Promise.resolve(() => {}),
  prefersReducedMotion: () => true,
}));

const { default: App } = await import("./App");

beforeEach(() => {
  vi.clearAllMocks();
  // Algún test cambia lo que contesta la búsqueda: cada test empieza con lo de siempre.
  commands.searchFiles.mockImplementation(defaultSearch);
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
      await screen.findByText("Pág. 37 ·", { exact: false }),
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

  it("un doble Enter no abre el Espacio dos veces", async () => {
    commands.runSpace.mockReturnValue(new Promise(() => {}));
    const { user, box } = await renderBar();
    await user.type(box, "curro");
    await user.keyboard("{Enter}{Enter}");
    await waitFor(() => expect(commands.runSpace).toHaveBeenCalledTimes(1));
    await user.keyboard("{Enter}");
    expect(commands.runSpace).toHaveBeenCalledTimes(1);
  });

  it("si la llamada para abrir un Espacio falla del todo, lo dice y la barra no se atasca", async () => {
    commands.runSpace.mockRejectedValueOnce(new Error("canal cerrado"));
    const { user, box } = await renderBar();
    await user.type(box, "curro");
    await user.keyboard("{Enter}");
    expect(await screen.findByText(/canal cerrado/)).toBeInTheDocument();
    // Se puede volver a intentar.
    commands.runSpace.mockReturnValue(ok({ status: "declined" }));
    await user.keyboard("{Escape}");
    await user.clear(box);
    await user.type(box, "curro");
    await user.keyboard("{Enter}");
    await waitFor(() => expect(commands.runSpace).toHaveBeenCalledTimes(2));
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

  describe("vista previa", () => {
    async function openPreview() {
      const { user, box } = await renderBar();
      await user.type(box, "contrato");
      await waitFor(() => expect(selectedText()).toContain("contrato.pdf"));
      return { user, box };
    }
    const region = (name: string) =>
      screen.findByRole("region", { name: `Vista previa de ${name}` });

    it("Tab abre la vista previa del resultado seleccionado", async () => {
      const { user } = await openPreview();
      await user.keyboard("{Tab}");
      const r = await region("contrato.pdf");
      expect(r).toHaveTextContent("pág. 37");
      expect(r).toHaveTextContent("primaria");
      expect(commands.previewFile).toHaveBeenCalledWith(12, 1, "contrato");
      expect(screen.queryByRole("listbox")).not.toBeInTheDocument();
    });

    it("↓ cambia de resultado sin salir de la vista previa", async () => {
      const { user } = await openPreview();
      await user.keyboard("{Tab}");
      await region("contrato.pdf");
      await user.keyboard("{ArrowDown}");
      expect(await region("notas.md")).toBeInTheDocument();
    });

    it("Esc y ← vuelven a la lista sin ocultar la barra", async () => {
      const { user } = await openPreview();
      await user.keyboard("{Tab}");
      await region("contrato.pdf");
      await user.keyboard("{Escape}");
      expect(screen.getByRole("listbox")).toBeInTheDocument();
      expect(screen.queryByRole("region")).not.toBeInTheDocument();
      expect(commands.hideLauncher).not.toHaveBeenCalled();
      await user.keyboard("{Tab}");
      await region("contrato.pdf");
      await user.keyboard("{ArrowLeft}");
      expect(screen.getByRole("listbox")).toBeInTheDocument();
      expect(commands.hideLauncher).not.toHaveBeenCalled();
    });

    it("Enter abre el archivo, en su página", async () => {
      const { user } = await openPreview();
      await user.keyboard("{Tab}");
      await region("contrato.pdf");
      await user.keyboard("{Enter}");
      expect(commands.openFile).toHaveBeenCalledWith(
        "C:\\Docs\\contrato.pdf",
        37,
      );
    });

    it("→ abre con el cursor al final, pero no con el cursor en medio", async () => {
      const { user, box } = await openPreview();
      box.setSelectionRange(3, 3);
      await user.keyboard("{ArrowRight}");
      expect(screen.queryByRole("region")).not.toBeInTheDocument();
      box.setSelectionRange(box.value.length, box.value.length);
      await user.keyboard("{ArrowRight}");
      expect(await region("contrato.pdf")).toBeInTheDocument();
    });

    it("escribir con la vista previa abierta vuelve a la lista", async () => {
      const { user } = await openPreview();
      await user.keyboard("{Tab}");
      await region("contrato.pdf");
      await user.keyboard("s");
      expect(screen.queryByRole("region")).not.toBeInTheDocument();
      expect(await screen.findByRole("listbox")).toBeInTheDocument();
    });

    describe("ratón y después teclado", () => {
      it("tras hacer clic en el texto de la vista previa, Esc vuelve a la lista", async () => {
        const { user } = await openPreview();
        await user.keyboard("{Tab}");
        const r = await region("contrato.pdf");
        await user.click(await screen.findByText("primaria"));
        await user.keyboard("{Escape}");
        expect(r).not.toBeInTheDocument();
        expect(screen.getByRole("listbox")).toBeInTheDocument();
        expect(commands.hideLauncher).not.toHaveBeenCalled();
      });

      it("con el foco fuera de la caja, Tab cierra la vista previa (y no la barra)", async () => {
        const { user, box } = await openPreview();
        await user.keyboard("{Tab}");
        await region("contrato.pdf");
        box.blur();
        expect(box).not.toHaveFocus();
        await user.keyboard("{Tab}");
        expect(screen.queryByRole("region")).not.toBeInTheDocument();
        expect(commands.hideLauncher).not.toHaveBeenCalled();
        // El foco vuelve a la caja: se puede seguir escribiendo.
        expect(box).toHaveFocus();
      });

      it("pasar el ratón por la lista y luego usar las flechas, Tab y Enter", async () => {
        const { user } = await openPreview();
        fireEvent.mouseMove(screen.getAllByRole("option")[1]);
        expect(selectedText()).toContain("notas.md");
        await user.keyboard("{ArrowUp}{Tab}");
        await region("contrato.pdf");
        await user.keyboard("{Enter}");
        expect(commands.openFile).toHaveBeenCalledWith(
          "C:\\Docs\\contrato.pdf",
          37,
        );
      });

      it("un clic en el fondo de la barra no le quita el foco a la caja", async () => {
        const { user, box } = await openPreview();
        await user.click(screen.getByRole("listbox"));
        expect(box).toHaveFocus();
        await user.keyboard("{Escape}");
        expect(commands.hideLauncher).toHaveBeenCalled();
      });
    });

    it("dice por qué sale: palabras, significado o nombre", async () => {
      const { user, box } = await renderBar();
      const answer = (h: ReturnType<typeof hit>) =>
        commands.searchFiles.mockImplementation((_q: string, seq: number) =>
          ok({ seq, results: { hits: [h], tookMs: 1 } }),
        );
      const base = hit("contrato.pdf");
      const cases: [ReturnType<typeof hit>, string][] = [
        [
          { ...base, snippet: [{ text: "primaria", hit: true }] },
          "Contiene «primaria»",
        ],
        [
          {
            ...base,
            snippet: [{ text: "primaria", hit: true }],
            byMeaning: true,
          },
          "Trata de lo mismo",
        ],
        [base, "Por el nombre"],
      ];
      for (const [h, label] of cases) {
        answer(h);
        await user.clear(box);
        await user.type(box, "clave");
        await waitFor(() => expect(selectedText()).toContain("contrato.pdf"));
        await user.keyboard("{Tab}");
        expect(await region("contrato.pdf")).toHaveTextContent(label);
        await user.keyboard("{Escape}");
      }
    });

    it("Tab con un Espacio seleccionado enseña sus acciones y Enter lo abre", async () => {
      commands.listSpaces.mockReturnValueOnce(
        ok([
          {
            ...space,
            space: {
              ...space.space,
              actions: [
                { kind: "openUrl", url: "https://example.com" },
                { kind: "wait", until: { kind: "seconds", secs: 3 } },
              ],
            },
          },
        ]),
      );
      commands.runSpace.mockReturnValue(ok({ status: "declined" }));
      const { user } = await renderBar();
      await screen.findByText("Espacios");
      await user.keyboard("{Tab}");
      const r = await region("Trabajo");
      expect(r).toHaveTextContent("Espacio · 2 acciones");
      const steps = screen.getByRole("list", { name: "Acciones, en orden" });
      expect(steps).toHaveTextContent("Abrir https://example.com");
      expect(steps).toHaveTextContent("Esperar 3 s");
      await user.keyboard("{Enter}");
      await waitFor(() => expect(commands.runSpace).toHaveBeenCalledWith(1));
    });

    it("con el ratón: la pista «Vista previa» la abre y ← vuelve a la lista", async () => {
      const { user } = await openPreview();
      await user.click(screen.getByRole("button", { name: /Vista previa/ }));
      await region("contrato.pdf");
      await user.click(
        screen.getByRole("button", { name: "Volver a la lista" }),
      );
      expect(screen.getByRole("listbox")).toBeInTheDocument();
      expect(commands.hideLauncher).not.toHaveBeenCalled();
    });

    it("con el ratón: las pistas «Abrir» y «Carpeta» hacen lo mismo que sus teclas", async () => {
      const { user, box } = await openPreview();
      await user.click(screen.getByRole("button", { name: /Carpeta/ }));
      expect(commands.revealFile).toHaveBeenCalledWith(
        "C:\\Docs\\contrato.pdf",
      );
      expect(box).toHaveFocus();
      await user.click(screen.getByRole("button", { name: "Abrir" }));
      expect(commands.openFile).toHaveBeenCalledWith(
        "C:\\Docs\\contrato.pdf",
        37,
      );
    });
  });
});
