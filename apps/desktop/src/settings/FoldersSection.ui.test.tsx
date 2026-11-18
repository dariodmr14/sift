// @vitest-environment jsdom
//
// Ajustes → Carpetas: la lista con su número de archivos, quitar con confirmación y añadir.

import "@testing-library/jest-dom/vitest";
import { cleanup, render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

const ok = <T,>(data: T) => Promise.resolve({ status: "ok" as const, data });

const roots = [
  {
    id: 1,
    path: "C:\\Users\\Ana\\Docs",
    files: 1240,
    errors: 0,
    lastIndexedAt: null,
    missing: false,
  },
  {
    id: 2,
    path: "D:\\Notas",
    files: 1,
    errors: 0,
    lastIndexedAt: null,
    missing: false,
  },
];

const commands = {
  listRoots: vi.fn(() => ok(roots)),
  indexStatus: vi.fn(() => Promise.resolve(null)),
  addRoot: vi.fn(() => ok(null)),
  removeRoot: vi.fn((_id: number) => ok(null)),
  reindex: vi.fn(() => ok(null)),
  unreadFiles: vi.fn(() => ok([])),
  getIndexMedia: vi.fn(() => Promise.resolve(true)),
  setIndexMedia: vi.fn(() => ok(true)),
  getIgnorePatterns: vi.fn(() => ok([] as string[])),
  setIgnorePatterns: vi.fn(() => ok([] as string[])),
};
const listen = () => Promise.resolve(() => {});
vi.mock("../bindings", () => ({
  commands,
  events: { indexProgress: { listen } },
}));

const { FoldersSection } = await import("./FoldersSection");

beforeEach(() => {
  vi.clearAllMocks();
  commands.listRoots.mockImplementation(() => ok(roots));
});
afterEach(cleanup);

describe("FoldersSection", () => {
  it("lists each folder with its number of files", async () => {
    render(<FoldersSection />);
    expect(await screen.findByText("Docs")).toBeInTheDocument();
    expect(screen.getByText("Notas")).toBeInTheDocument();
    expect(screen.getByText(/1\.240 archivos/)).toBeInTheDocument();
    expect(screen.getByText(/^1 archivo/)).toBeInTheDocument();
  });

  it("says so when there are no folders", async () => {
    commands.listRoots.mockImplementation(() => ok([]));
    render(<FoldersSection />);
    expect(await screen.findByText(/Aún no hay carpetas/)).toBeInTheDocument();
    expect(
      screen.getByText("Añade una carpeta para empezar."),
    ).toBeInTheDocument();
  });

  it("«Quitar» asks first; «No» cancels without removing", async () => {
    const user = userEvent.setup();
    render(<FoldersSection />);
    await user.click(
      await screen.findByRole("button", { name: "Quitar D:\\Notas" }),
    );
    expect(screen.getByText("¿Dejar de buscar aquí?")).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "No" }));
    expect(commands.removeRoot).not.toHaveBeenCalled();
    expect(
      screen.queryByText("¿Dejar de buscar aquí?"),
    ).not.toBeInTheDocument();
  });

  it("confirming removes that folder by id and reloads the list", async () => {
    const user = userEvent.setup();
    render(<FoldersSection />);
    await user.click(
      await screen.findByRole("button", { name: "Quitar D:\\Notas" }),
    );
    commands.listRoots.mockClear();
    await user.click(screen.getByRole("button", { name: "Sí, quitar" }));
    expect(commands.removeRoot).toHaveBeenCalledTimes(1);
    expect(commands.removeRoot).toHaveBeenCalledWith(2);
    expect(commands.listRoots).toHaveBeenCalled();
  });

  it("«Añadir carpeta…» calls addRoot and reloads", async () => {
    const user = userEvent.setup();
    render(<FoldersSection />);
    await screen.findByText("Docs");
    commands.listRoots.mockClear();
    await user.click(screen.getByRole("button", { name: /Añadir carpeta/ }));
    expect(commands.addRoot).toHaveBeenCalledTimes(1);
    expect(commands.listRoots).toHaveBeenCalled();
  });

  it("a missing folder says so instead of the file count", async () => {
    commands.listRoots.mockImplementation(() =>
      ok([{ ...roots[1], missing: true }]),
    );
    render(<FoldersSection />);
    const row = (await screen.findByText("Notas")).closest("li");
    expect(row).not.toBeNull();
    expect(
      within(row as HTMLElement).getByText(/No se encuentra la carpeta/),
    ).toBeInTheDocument();
  });
});
