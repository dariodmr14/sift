// @vitest-environment jsdom
//
// El indicador de carga: las barras son decorativas, el texto se anuncia y, con retraso,
// lo rápido no llega a enseñarlo.

import "@testing-library/jest-dom/vitest";
import { act, cleanup, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { SiftLoader, SiftLoading } from "./SiftLoader";

afterEach(() => {
  cleanup();
  vi.useRealTimers();
});

describe("SiftLoader", () => {
  it("tres barras ocultas al lector de pantalla", () => {
    const { container } = render(<SiftLoader />);
    const loader = container.querySelector(".sift-loader");
    expect(loader).toHaveAttribute("aria-hidden");
    expect(loader?.childElementCount).toBe(3);
  });

  it("con texto, se anuncia; con retraso, no sale antes de tiempo", () => {
    vi.useFakeTimers();
    render(<SiftLoading label="Preparando la vista previa…" delay={300} />);
    expect(screen.queryByRole("status")).not.toBeInTheDocument();
    act(() => void vi.advanceTimersByTime(300));
    expect(screen.getByRole("status")).toHaveTextContent(
      "Preparando la vista previa…",
    );
  });
});
