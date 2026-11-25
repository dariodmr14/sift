// @vitest-environment jsdom
import "@testing-library/jest-dom/vitest";
import { cleanup, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { GlassBlocker } from "../bindings";

const ok = <T,>(data: T) => Promise.resolve({ status: "ok" as const, data });
let blocker: GlassBlocker | null = null;
const commands = {
  setTheme: vi.fn((theme: string) => ok(theme)),
  setIconsHidden: vi.fn((hidden: boolean) => ok(hidden)),
  setGlassDisabled: vi.fn((disabled: boolean) => ok(disabled)),
  getGlassBlocker: vi.fn(() => Promise.resolve(blocker)),
};
vi.mock("../bindings", () => ({
  commands,
  events: { appearanceChanged: { listen: () => Promise.resolve(() => {}) } },
}));

const { AppearanceSection } = await import("./AppearanceSection");

afterEach(cleanup);
beforeEach(() => {
  vi.clearAllMocks();
  blocker = null;
});

describe("Apariencia", () => {
  it("el tema empieza como Windows y se guarda al elegir otro", async () => {
    render(
      <AppearanceSection
        theme="system"
        iconsHidden={false}
        disableGlass={false}
      />,
    );
    const select = screen.getByRole("combobox", { name: "Tema" });
    expect(select).toHaveValue("system");
    expect(screen.getAllByRole("option").map((o) => o.textContent)).toEqual([
      "Como Windows",
      "Claro",
      "Oscuro",
    ]);

    await userEvent.selectOptions(select, "dark");
    expect(commands.setTheme).toHaveBeenCalledWith("dark");
    expect(select).toHaveValue("dark");
  });

  it("explica por qué no hay vidrio cuando Windows no puede pintarlo", async () => {
    blocker = "sandbox";
    render(
      <AppearanceSection
        theme="light"
        iconsHidden={false}
        disableGlass={false}
      />,
    );
    expect(
      await screen.findByText(/Windows Sandbox.*fondo opaco/),
    ).toBeInTheDocument();
  });

  it("con el vidrio desactivado a mano no hace falta explicarlo", async () => {
    blocker = "batterySaver";
    render(
      <AppearanceSection theme="light" iconsHidden={false} disableGlass />,
    );
    await waitFor(() => expect(commands.getGlassBlocker).toHaveBeenCalled());
    expect(screen.queryByRole("status")).not.toBeInTheDocument();
  });
});
