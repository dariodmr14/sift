// @vitest-environment jsdom
//
// Las acciones «Ejecutar comando» y «Abrir web»: cada campo llega por separado al `onChange`
// (el programa, los argumentos y la carpeta nunca se juntan en una sola cadena).

import "@testing-library/jest-dom/vitest";
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { Action } from "./draft";

const ok = <T,>(data: T) => Promise.resolve({ status: "ok" as const, data });

const commands = {
  listMonitors: vi.fn(() => ok([] as unknown[])),
  suggestPaths: vi.fn((_t: string, _k: string) => ok([] as string[])),
  pickPath: vi.fn((_k: string) => ok(null as string | null)),
  testAction: vi.fn(),
  checkWait: vi.fn(),
};
vi.mock("../bindings", () => ({ commands }));

// jsdom no trae `matchMedia` (lo usa el arrastre de acciones).
vi.stubGlobal("matchMedia", (query: string) => ({
  matches: false,
  media: query,
  addEventListener: () => {},
  removeEventListener: () => {},
}));

const { ActionEditor } = await import("./ActionEditor");

function setup(action: Action) {
  const onChange = vi.fn();
  render(
    <ul>
      <ActionEditor
        action={action}
        index={0}
        count={1}
        onChange={onChange}
        onMove={vi.fn()}
        onRemove={vi.fn()}
        sortableId="a"
      />
    </ul>,
  );
  return onChange;
}

type Command = Extract<Action, { kind: "runCommand" }>;
const command = (over: Partial<Command> = {}): Action => ({
  kind: "runCommand",
  program: "git",
  args: ["status"],
  cwd: null,
  ...over,
});

beforeEach(() => vi.clearAllMocks());
afterEach(cleanup);

describe("ActionEditor: Ejecutar comando", () => {
  it("the program goes alone, with args and cwd untouched", async () => {
    const user = userEvent.setup();
    const onChange = setup(command({ program: "", cwd: "C:\\Work" }));
    await user.type(screen.getByPlaceholderText("docker"), "x");
    expect(onChange).toHaveBeenLastCalledWith(
      command({ program: "x", cwd: "C:\\Work" }),
    );
  });

  it("each line of the textarea is one argument (never joined)", () => {
    const onChange = setup(command({ args: [] }));
    fireEvent.change(screen.getByPlaceholderText(/compose/), {
      target: { value: "compose\nup -d" },
    });
    expect(onChange).toHaveBeenLastCalledWith(
      command({ args: ["compose", "up -d"] }),
    );
  });

  it("existing args show one per line", () => {
    setup(command({ args: ["compose", "up"] }));
    expect(screen.getByPlaceholderText(/compose/)).toHaveValue("compose\nup");
  });

  it("the working folder is its own field; empty becomes null", async () => {
    const user = userEvent.setup();
    const onChange = setup(command({ cwd: "C:\\a" }));
    const cwd = screen.getByLabelText("Carpeta de trabajo");
    await user.type(cwd, "b");
    expect(onChange).toHaveBeenLastCalledWith(command({ cwd: "C:\\ab" }));
    onChange.mockClear();
    await user.clear(cwd);
    expect(onChange).toHaveBeenLastCalledWith(command({ cwd: null }));
  });
});

describe("ActionEditor: Abrir web", () => {
  it("what you type arrives as the url", async () => {
    const user = userEvent.setup();
    const onChange = setup({ kind: "openUrl", url: "https://a" });
    await user.type(screen.getByLabelText("URL"), "b");
    expect(onChange).toHaveBeenLastCalledWith({
      kind: "openUrl",
      url: "https://ab",
    });
  });
});
