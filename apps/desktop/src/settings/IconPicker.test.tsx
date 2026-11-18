// @vitest-environment jsdom
//
// El selector de icono: cada pestaña manda al `onChange` el icono que toca. La librería
// (Lucide) se carga de verdad con un `import()` perezoso; solo se mira que filtre.

import "@testing-library/jest-dom/vitest";
import { cleanup, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { Icon } from "./draft";

const ok = <T,>(data: T) => Promise.resolve({ status: "ok" as const, data });

const commands = {
  pickIconImage: vi.fn(() => ok(null as Icon | null)),
};
vi.mock("../bindings", () => ({ commands }));

const { IconPicker } = await import("./IconPicker");

function setup(icon: Icon = { kind: "auto" }) {
  const onChange = vi.fn<(icon: Icon) => void>();
  render(<IconPicker icon={icon} name="Clase" onChange={onChange} />);
  return onChange;
}

const tab = (name: string) => screen.getByRole("tab", { name });

beforeEach(() => vi.clearAllMocks());
afterEach(cleanup);

describe("IconPicker", () => {
  it("«Ninguno» and «Automático» send their icon", async () => {
    const user = userEvent.setup();
    const onChange = setup({ kind: "emoji", value: "🚀" });
    await user.click(tab("Ninguno"));
    expect(onChange).toHaveBeenLastCalledWith({ kind: "none" });
    await user.click(tab("Automático"));
    expect(onChange).toHaveBeenLastCalledWith({ kind: "auto" });
  });

  it("the current kind starts as the selected tab", () => {
    setup({ kind: "emoji", value: "🚀" });
    expect(tab("Emoji")).toHaveAttribute("aria-selected", "true");
    expect(tab("Automático")).toHaveAttribute("aria-selected", "false");
  });

  it("«Emoji»: choosing one sends it, and writing one too", async () => {
    const user = userEvent.setup();
    const onChange = setup();
    await user.click(tab("Emoji"));
    expect(onChange).not.toHaveBeenCalled(); // cambiar de pestaña no cambia el icono
    await user.click(screen.getByRole("button", { name: "📚" }));
    expect(onChange).toHaveBeenLastCalledWith({ kind: "emoji", value: "📚" });
    await user.type(screen.getByPlaceholderText(/O escribe uno/), "X");
    expect(onChange).toHaveBeenLastCalledWith({ kind: "emoji", value: "X" });
  });

  it("«Emoji»: emptying the field goes back to automatic", async () => {
    const user = userEvent.setup();
    const onChange = setup({ kind: "emoji", value: "🚀" });
    await user.clear(screen.getByPlaceholderText(/O escribe uno/));
    expect(onChange).toHaveBeenLastCalledWith({ kind: "auto" });
  });

  it("«Imagen»: sends what the native picker returns", async () => {
    const user = userEvent.setup();
    const image: Icon = { kind: "image", mime: "image/png", data: "AAAA" };
    commands.pickIconImage.mockImplementationOnce(() => ok(image));
    const onChange = setup();
    await user.click(tab("Imagen"));
    await user.click(screen.getByRole("button", { name: "Elegir imagen…" }));
    expect(onChange).toHaveBeenCalledWith(image);
  });

  it("«Imagen»: a cancelled picker changes nothing; an error is shown", async () => {
    const user = userEvent.setup();
    const onChange = setup();
    await user.click(tab("Imagen"));
    await user.click(screen.getByRole("button", { name: "Elegir imagen…" }));
    expect(onChange).not.toHaveBeenCalled();
    commands.pickIconImage.mockImplementationOnce(
      () =>
        Promise.resolve({
          status: "error" as const,
          error: { message: "Imagen demasiado grande" },
        }) as never,
    );
    await user.click(screen.getByRole("button", { name: "Elegir imagen…" }));
    expect(await screen.findByText("Imagen demasiado grande")).toBeVisible();
  });

  it("«Librería»: loads the icons, filters them and sends the chosen one", async () => {
    const user = userEvent.setup();
    const onChange = setup();
    await user.click(tab("Librería"));
    const icons = () =>
      screen
        .getAllByRole("button", { pressed: false })
        .filter((b) => b.getAttribute("title"));
    await waitFor(() => expect(icons().length).toBeGreaterThan(0));
    const before = icons().length;
    await user.type(screen.getByPlaceholderText("Buscar icono"), "folder");
    expect(icons().length).toBeLessThan(before);
    const first = icons()[0];
    await user.click(first);
    const [arg] = onChange.mock.lastCall ?? [];
    expect(arg).toMatchObject({ kind: "library" });
    expect(arg).toEqual(
      expect.objectContaining({
        name: expect.stringMatching(/Folder/) as string,
        svg: expect.stringContaining("<svg") as string,
      }),
    );
  });

  it("«Librería»: says so when nothing matches", async () => {
    const user = userEvent.setup();
    setup();
    await user.click(tab("Librería"));
    await user.type(
      await screen.findByPlaceholderText("Buscar icono"),
      "zzzzqqqq",
    );
    expect(
      await screen.findByText("Ningún icono con ese nombre"),
    ).toBeVisible();
  });
});
