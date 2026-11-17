// Atrapa el foco dentro de un diálogo (F12): al abrirse el foco entra, Tab y Mayús+Tab dan
// la vuelta sin salir, y al cerrarse vuelve a lo que lo tenía.

import { useEffect, useState, type RefObject } from "react";

const FOCUSABLE =
  'a[href], button:not([disabled]), input:not([disabled]), select:not([disabled]), textarea:not([disabled]), [tabindex]:not([tabindex="-1"])';

function focusables(root: HTMLElement): HTMLElement[] {
  return Array.from(root.querySelectorAll<HTMLElement>(FOCUSABLE)).filter(
    // jsdom no maqueta, así que no se puede mirar el tamaño: basta con `hidden`.
    (el) => !el.hidden && !el.closest("[hidden]"),
  );
}

export function useFocusTrap(ref: RefObject<HTMLElement | null>): void {
  // A quién devolver el foco: se lee al renderizar por primera vez, antes de que React
  // enfoque un `autoFocus` de dentro (lo hace al montar, antes que los efectos).
  const [previous] = useState(
    () => document.activeElement as HTMLElement | null,
  );

  useEffect(() => {
    const root = ref.current;
    if (!root) return;

    // Si un campo con `autoFocus` ya tiene el foco, se queda ahí. React no escribe el
    // atributo `autofocus` en el DOM, así que no se puede buscar con un selector.
    if (!root.contains(document.activeElement)) {
      (focusables(root)[0] ?? root).focus();
    }

    const onKeyDown = (e: KeyboardEvent) => {
      if (e.key !== "Tab") return;
      const items = focusables(root);
      if (items.length === 0) {
        e.preventDefault();
        return;
      }
      const first = items[0];
      const last = items[items.length - 1];
      const active = document.activeElement;
      if (e.shiftKey && (active === first || active === root)) {
        e.preventDefault();
        last.focus();
      } else if (!e.shiftKey && active === last) {
        e.preventDefault();
        first.focus();
      }
    };
    root.addEventListener("keydown", onKeyDown);

    return () => {
      root.removeEventListener("keydown", onKeyDown);
      if (previous && previous.isConnected) previous.focus();
    };
  }, [ref, previous]);
}
