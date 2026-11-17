// @vitest-environment jsdom

import "@testing-library/jest-dom/vitest";
import { cleanup, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { useState } from "react";
import { afterEach, describe, expect, it } from "vitest";
import { Button, Modal, TextInput } from "../settings/ui";

afterEach(cleanup);

function Host({ autoFocusField = false }: { autoFocusField?: boolean }) {
  const [open, setOpen] = useState(false);
  return (
    <>
      <button type="button" onClick={() => setOpen(true)}>
        Abrir
      </button>
      {open && (
        <Modal title="Prueba" onClose={() => setOpen(false)}>
          <TextInput aria-label="Nombre" autoFocus={autoFocusField} />
          <Button>Aceptar</Button>
        </Modal>
      )}
    </>
  );
}

describe("useFocusTrap", () => {
  it("entra, da la vuelta con Tab y Mayús+Tab, y devuelve el foco al cerrar", async () => {
    const user = userEvent.setup();
    render(<Host />);
    const opener = screen.getByRole("button", { name: "Abrir" });
    await user.click(opener);

    const dialog = screen.getByRole("dialog", { name: "Prueba" });
    const first = screen.getByRole("button", { name: "Cerrar" });
    const last = screen.getByRole("button", { name: "Aceptar" });
    expect(dialog).toContainElement(document.activeElement as HTMLElement);
    expect(first).toHaveFocus();

    last.focus();
    await user.tab();
    expect(first).toHaveFocus();

    await user.tab({ shift: true });
    expect(last).toHaveFocus();

    await user.keyboard("{Escape}");
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
    expect(opener).toHaveFocus();
  });

  it("respeta el campo con autoFocus y aun así devuelve el foco a quien lo abrió", async () => {
    const user = userEvent.setup();
    render(<Host autoFocusField />);
    const opener = screen.getByRole("button", { name: "Abrir" });
    await user.click(opener);

    expect(screen.getByRole("textbox", { name: "Nombre" })).toHaveFocus();

    await user.keyboard("{Escape}");
    expect(opener).toHaveFocus();
  });
});
