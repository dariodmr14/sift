// @vitest-environment jsdom
//
// La red de seguridad de cada ventana: si algo falla al pintar, se dice y se puede seguir
// (Esc oculta la barra); y la barra solo se cierra con su propia animación de salida.

import "@testing-library/jest-dom/vitest";
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

const hideLauncher = vi.fn(() => Promise.resolve());
vi.mock("../bindings", () => ({ commands: { hideLauncher } }));
const { ErrorBoundary } = await import("./ErrorBoundary");
const { dismissLauncher } = await import("../useLauncher");

afterEach(() => {
  cleanup();
  vi.useRealTimers();
  hideLauncher.mockClear();
});

function Boom(): never {
  throw new Error("fallo al pintar");
}

describe("ErrorBoundary", () => {
  it("enseña un aviso en vez de dejar la ventana en blanco; Esc oculta la barra", () => {
    vi.spyOn(console, "error").mockImplementation(() => {});
    const onEscape = vi.fn();
    render(
      <ErrorBoundary onEscape={onEscape}>
        <Boom />
      </ErrorBoundary>,
    );
    expect(screen.getByRole("alert")).toHaveTextContent(
      "Algo ha fallado en esta ventana",
    );
    expect(
      screen.getByRole("button", { name: "Volver a cargar" }),
    ).toBeInTheDocument();
    fireEvent.keyDown(window, { key: "Escape" });
    expect(onEscape).toHaveBeenCalled();
  });
});

describe("dismissLauncher", () => {
  const anim = (target: HTMLElement, name: string) => {
    const e = new Event("animationend", { bubbles: true });
    Object.defineProperty(e, "animationName", { value: name });
    target.dispatchEvent(e);
  };

  it("solo se oculta al acabar su propia salida, no la de lo que hay dentro", () => {
    window.matchMedia = vi.fn().mockReturnValue({ matches: false }) as never;
    const main = document.createElement("main");
    const child = document.createElement("span");
    main.appendChild(child);
    document.body.appendChild(main);
    dismissLauncher(main);
    anim(child, "sift-sieve");
    expect(hideLauncher).not.toHaveBeenCalled();
    anim(main, "sift-exit");
    expect(hideLauncher).toHaveBeenCalledTimes(1);
    main.remove();
  });

  it("si la animación no termina nunca, se oculta igual al poco", () => {
    vi.useFakeTimers();
    window.matchMedia = vi.fn().mockReturnValue({ matches: false }) as never;
    const main = document.createElement("main");
    dismissLauncher(main);
    vi.advanceTimersByTime(400);
    expect(hideLauncher).toHaveBeenCalledTimes(1);
  });
});
