// @vitest-environment jsdom
//
// ActionEditor, más casos: «Abrir aplicación» con su selector, el botón «Probar» y las
// esperas (cada tipo enseña solo su campo y lo escrito llega al `onChange`).

import "@testing-library/jest-dom/vitest";
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { Action } from "./draft";

const ok = <T,>(data: T) => Promise.resolve({ status: "ok" as const, data });
const fail = (message: string) =>
  Promise.resolve({ status: "error" as const, error: { message } }) as never;

const commands = {
  listMonitors: vi.fn(() => ok([] as unknown[])),
  suggestPaths: vi.fn((_t: string, _k: string) => ok([] as string[])),
  pickPath: vi.fn((_k: string) => ok(null as string | null)),
  testAction: vi.fn((_a: unknown) =>
    ok({ status: "done" as const, outcomes: [] as unknown[] }),
  ),
  checkWait: vi.fn((_a: unknown) => ok("responde ahora mismo")),
  listApps: vi.fn(() =>
    ok([
      { id: "app.bloc", name: "Bloc de notas" },
      { id: "app.calc", name: "Calculadora" },
    ]),
  ),
  getIcons: vi.fn((_k: string[]) => ok([] as (string | null)[])),
};
vi.mock("../bindings", () => ({ commands }));

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

type Wait = Extract<Action, { kind: "wait" }>;
const wait = (until: Wait["until"], over: Partial<Wait> = {}): Action => ({
  kind: "wait",
  until,
  timeoutSecs: 60,
  onTimeout: "stop",
  ...over,
});

const emptyApp: Action = { kind: "openApp", appId: "", name: "" };

beforeEach(() => vi.clearAllMocks());
afterEach(cleanup);

describe("ActionEditor: Abrir aplicación", () => {
  it("shows the chosen app and offers «Cambiar…»", () => {
    setup({ kind: "openApp", appId: "app.bloc", name: "Bloc de notas" });
    expect(screen.getByText("Bloc de notas")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Cambiar…" })).toBeVisible();
  });

  it("without an app it says so and offers «Elegir aplicación…»", () => {
    setup(emptyApp);
    expect(screen.getByText("Ninguna aplicación elegida")).toBeVisible();
    expect(
      screen.getByRole("button", { name: "Elegir aplicación…" }),
    ).toBeVisible();
  });

  it("picking an app in the picker sends its id and name", async () => {
    const user = userEvent.setup();
    const onChange = setup(emptyApp);
    await user.click(
      screen.getByRole("button", { name: "Elegir aplicación…" }),
    );
    await user.click(
      await screen.findByRole("option", { name: "Calculadora" }),
    );
    expect(onChange).toHaveBeenCalledWith({
      kind: "openApp",
      appId: "app.calc",
      name: "Calculadora",
    });
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
  });

  it("the picker filters by name and Enter picks the highlighted app", async () => {
    const user = userEvent.setup();
    const onChange = setup(emptyApp);
    await user.click(
      screen.getByRole("button", { name: "Elegir aplicación…" }),
    );
    await screen.findByRole("option", { name: "Bloc de notas" });
    await user.type(screen.getByPlaceholderText("Buscar aplicación"), "calc");
    expect(screen.queryByRole("option", { name: "Bloc de notas" })).toBeNull();
    await user.keyboard("{Enter}");
    expect(onChange).toHaveBeenCalledWith({
      kind: "openApp",
      appId: "app.calc",
      name: "Calculadora",
    });
  });

  it("the picker says when nothing matches and Escape closes it", async () => {
    const user = userEvent.setup();
    const onChange = setup(emptyApp);
    await user.click(
      screen.getByRole("button", { name: "Elegir aplicación…" }),
    );
    await screen.findByRole("option", { name: "Calculadora" });
    await user.type(screen.getByPlaceholderText("Buscar aplicación"), "zzz");
    expect(screen.getByText("Ninguna aplicación con ese nombre")).toBeVisible();
    await user.keyboard("{Escape}");
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
    expect(onChange).not.toHaveBeenCalled();
  });

  it("the picker shows the error when the list cannot be read", async () => {
    commands.listApps.mockImplementationOnce(() => fail("No se pudo leer"));
    const user = userEvent.setup();
    setup(emptyApp);
    await user.click(
      screen.getByRole("button", { name: "Elegir aplicación…" }),
    );
    expect(await screen.findByText("No se pudo leer")).toBeVisible();
  });
});

describe("ActionEditor: «Probar»", () => {
  const url: Action = { kind: "openUrl", url: "https://a" };

  it("calls testAction with the action and says «Funciona.»", async () => {
    const user = userEvent.setup();
    commands.testAction.mockImplementationOnce(() =>
      ok({
        status: "done" as const,
        outcomes: [{ index: 1, label: "Abrir", ran: true }],
      }),
    );
    setup(url);
    await user.click(screen.getByRole("button", { name: "Probar" }));
    expect(commands.testAction).toHaveBeenCalledWith(url);
    expect(await screen.findByText("Funciona.")).toBeVisible();
  });

  it("shows the error of the action that failed", async () => {
    const user = userEvent.setup();
    commands.testAction.mockImplementationOnce(() =>
      ok({
        status: "done" as const,
        outcomes: [{ index: 1, label: "Abrir", ran: true, error: "No existe" }],
      }),
    );
    setup(url);
    await user.click(screen.getByRole("button", { name: "Probar" }));
    expect(await screen.findByText("No existe")).toBeVisible();
  });

  it("shows the error when the command itself fails", async () => {
    const user = userEvent.setup();
    commands.testAction.mockImplementationOnce(() => fail("Algo falló"));
    setup(url);
    await user.click(screen.getByRole("button", { name: "Probar" }));
    expect(await screen.findByText("Algo falló")).toBeVisible();
  });

  it("says nothing was run when the confirmation is declined", async () => {
    const user = userEvent.setup();
    commands.testAction.mockImplementationOnce(
      () => ok({ status: "declined" as const }) as never,
    );
    setup(url);
    await user.click(screen.getByRole("button", { name: "Probar" }));
    expect(
      await screen.findByText("No se ejecutó: cancelaste la confirmación."),
    ).toBeVisible();
  });

  it("a wait is checked with checkWait and shows what was seen", async () => {
    const user = userEvent.setup();
    const action = wait({ kind: "webResponds", url: "http://localhost:80" });
    setup(action);
    await user.click(screen.getByRole("button", { name: "Probar" }));
    expect(commands.checkWait).toHaveBeenCalledWith(action);
    expect(commands.testAction).not.toHaveBeenCalled();
    expect(await screen.findByText("responde ahora mismo")).toBeVisible();
  });

  it("a wait that is not met shows its reason", async () => {
    const user = userEvent.setup();
    commands.checkWait.mockImplementationOnce(() => fail("No responde"));
    setup(wait({ kind: "serviceAccepts", port: 5432 }));
    await user.click(screen.getByRole("button", { name: "Probar" }));
    expect(await screen.findByText("No responde")).toBeVisible();
  });

  it("«Probar» is disabled for waits with nothing to check ahead of time", () => {
    setup(wait({ kind: "seconds", secs: 5 }));
    expect(screen.getByRole("button", { name: "Probar" })).toBeDisabled();
  });
});

describe("ActionEditor: Esperar", () => {
  const web = wait({ kind: "webResponds", url: "http://localhost:80" });

  it("web: shows the address field and sends what is typed", async () => {
    const user = userEvent.setup();
    const onChange = setup(web);
    expect(screen.queryByPlaceholderText("5432")).toBeNull();
    await user.type(screen.getByPlaceholderText("http://localhost:8080"), "1");
    expect(onChange).toHaveBeenLastCalledWith(
      wait({ kind: "webResponds", url: "http://localhost:801" }),
    );
  });

  it("service: shows only the port and sends it as a number", () => {
    const onChange = setup(wait({ kind: "serviceAccepts", port: 0 }));
    expect(screen.queryByPlaceholderText("http://localhost:8080")).toBeNull();
    fireEvent.change(screen.getByPlaceholderText("5432"), {
      target: { value: "5432" },
    });
    expect(onChange).toHaveBeenLastCalledWith(
      wait({ kind: "serviceAccepts", port: 5432 }),
    );
  });

  it("previous command: shows only its explanation", () => {
    setup(wait({ kind: "commandFinished" }, { timeoutSecs: 120 }));
    expect(screen.getByText(/Espera a que acabe el comando/)).toBeVisible();
    expect(screen.queryByPlaceholderText("http://localhost:8080")).toBeNull();
    expect(screen.queryByPlaceholderText("5432")).toBeNull();
    expect(screen.getByText(/segundos o falla/)).toBeVisible();
  });

  it("seconds: shows only the seconds field, without a timeout", () => {
    const onChange = setup(wait({ kind: "seconds", secs: 5 }));
    expect(screen.queryByLabelText("Tiempo límite en segundos")).toBeNull();
    fireEvent.change(screen.getByLabelText("Segundos"), {
      target: { value: "12" },
    });
    expect(onChange).toHaveBeenLastCalledWith(
      wait({ kind: "seconds", secs: 12 }),
    );
  });

  it("changing the kind sends the new wait with its defaults", () => {
    const onChange = setup(web);
    fireEvent.change(screen.getByLabelText("A qué esperar"), {
      target: { value: "seconds" },
    });
    expect(onChange).toHaveBeenLastCalledWith(
      wait({ kind: "seconds", secs: 5 }),
    );
    fireEvent.change(screen.getByLabelText("A qué esperar"), {
      target: { value: "commandFinished" },
    });
    expect(onChange).toHaveBeenLastCalledWith(
      wait({ kind: "commandFinished" }, { timeoutSecs: 120 }),
    );
  });

  it("the timeout and what to do on timeout reach onChange", () => {
    const onChange = setup(web);
    fireEvent.change(screen.getByLabelText("Tiempo límite en segundos"), {
      target: { value: "30" },
    });
    expect(onChange).toHaveBeenLastCalledWith({ ...web, timeoutSecs: 30 });
    fireEvent.change(screen.getByLabelText("Qué hacer si no ocurre"), {
      target: { value: "continue" },
    });
    expect(onChange).toHaveBeenLastCalledWith({
      ...web,
      onTimeout: "continue",
    });
  });
});
