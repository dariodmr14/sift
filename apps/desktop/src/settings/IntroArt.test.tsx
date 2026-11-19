// @vitest-environment jsdom
//
// Las ilustraciones de la introducción: decorativas, una por paso, y con «reducir
// movimiento» enseñan directamente su último fotograma.

import "@testing-library/jest-dom/vitest";
import { act, cleanup, render } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { IntroArt } from "./IntroArt";
import { INTRO_STEPS } from "./introSteps";

function reduce(on: boolean) {
  window.matchMedia = vi.fn().mockReturnValue({ matches: on }) as never;
}

afterEach(() => {
  cleanup();
  vi.useRealTimers();
});

describe("IntroArt", () => {
  it.each(INTRO_STEPS.map((s) => s.id))(
    "el paso %s tiene su ilustración, oculta al lector de pantalla",
    (id) => {
      reduce(false);
      const { container } = render(
        <IntroArt id={id} keys={["Ctrl", "Alt", "Espacio"]} />,
      );
      const art = container.querySelector(`[data-art="${id}"]`);
      expect(art).toHaveAttribute("aria-hidden");
      expect(art?.childElementCount).toBeGreaterThan(0);
    },
  );

  it("el texto se escribe solo y el contador sube hasta el final", () => {
    vi.useFakeTimers();
    reduce(false);
    const search = render(<IntroArt id="search" keys={[]} />);
    expect(search.container.textContent).not.toContain("factura");
    act(() => void vi.advanceTimersByTime(2000));
    expect(search.container.textContent).toContain("factura");
    const folders = render(<IntroArt id="folders" keys={[]} />);
    expect(folders.container.textContent).toContain("0 archivos");
    act(() => void vi.advanceTimersByTime(2000));
    expect(folders.container.textContent).toContain("1284 archivos");
  });

  it("con «reducir movimiento», el último fotograma desde el principio", () => {
    reduce(true);
    const { container } = render(<IntroArt id="search" keys={[]} />);
    expect(container.textContent).toContain("factura");
  });
});
