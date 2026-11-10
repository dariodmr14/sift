import { useEffect, useRef, useState } from "react";
import { dropSlot, isNoop } from "./reorder";

/** Píxeles que hay que mover antes de que empiece el arrastre (un clic no se mueve). */
const THRESHOLD = 4;

interface Drag {
  from: number;
  slot: number;
  /** ¿Ya se ha movido lo suficiente para ser un arrastre (y no un clic)? */
  active: boolean;
}

/**
 * Reordenar arrastrando con eventos de puntero.
 *
 * No usa el drag & drop de HTML5: en Tauri para Windows la ventana intercepta el arrastre
 * (para recibir archivos soltados) y esos eventos no llegan bien a la página. Con
 * eventos de puntero y `setPointerCapture` funciona igual con ratón, lápiz o táctil.
 *
 * Uso: `itemRef(i)` en cada elemento (para medirlo), `handleProps(i)` en lo que se coge
 * (la fila entera o un asa), y `slot` para pintar dónde caerá. `onDrop(from, slot)` se
 * llama al soltar si el orden cambia. Esc cancela.
 */
export function useDragReorder(onDrop: (from: number, slot: number) => void) {
  const [drag, setDrag] = useState<Drag | null>(null);
  const items = useRef<(HTMLElement | null)[]>([]);
  // Dónde empezó el arrastre. El origen se guarda aquí, no se deduce del elemento que
  // recibe cada evento: con un movimiento rápido el puntero pasa por encima de otros.
  const start = useRef<{ y: number; pointer: number; from: number } | null>(
    null,
  );
  // Tras un arrastre, el navegador dispara también un `click`: hay que ignorarlo.
  const swallowClick = useRef(false);

  useEffect(() => {
    if (!drag?.active) return;
    const cancel = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        e.preventDefault();
        start.current = null;
        setDrag(null);
      }
    };
    window.addEventListener("keydown", cancel);
    return () => window.removeEventListener("keydown", cancel);
  }, [drag?.active]);

  const itemRef = (i: number) => (el: HTMLElement | null) => {
    items.current[i] = el;
  };

  const handleProps = (i: number) => ({
    onPointerDown(e: React.PointerEvent<HTMLElement>) {
      if (e.button !== 0) return; // solo el botón principal
      start.current = { y: e.clientY, pointer: e.pointerId, from: i };
      // Captura desde el primer instante: todos los movimientos llegan a este elemento
      // aunque el puntero salga de él (el `click` sigue funcionando si no se mueve).
      e.currentTarget.setPointerCapture(e.pointerId);
      setDrag({ from: i, slot: i, active: false });
    },
    onPointerMove(e: React.PointerEvent<HTMLElement>) {
      if (!start.current || start.current.pointer !== e.pointerId) return;
      const moved = Math.abs(e.clientY - start.current.y) >= THRESHOLD;
      if (!moved && !drag?.active) return;
      const { from } = start.current;
      const boxes = items.current.map((el) => {
        const r = el?.getBoundingClientRect();
        return { top: r?.top ?? 0, height: r?.height ?? 0 };
      });
      setDrag({ from, slot: dropSlot(e.clientY, boxes), active: true });
    },
    onPointerUp() {
      if (drag?.active) {
        swallowClick.current = true;
        if (!isNoop(drag.from, drag.slot)) onDrop(drag.from, drag.slot);
      }
      start.current = null;
      setDrag(null);
    },
    onPointerCancel() {
      start.current = null;
      setDrag(null);
    },
    onClickCapture(e: React.MouseEvent) {
      if (swallowClick.current) {
        swallowClick.current = false;
        e.preventDefault();
        e.stopPropagation();
      }
    },
  });

  /** Clases para el elemento `i`: atenuado si se arrastra, línea donde caerá. */
  const itemClass = (i: number, count: number) => {
    if (!drag?.active) return "";
    const classes: string[] = [];
    if (i === drag.from) classes.push("sift-dragging");
    if (!isNoop(drag.from, drag.slot)) {
      if (drag.slot === i) classes.push("sift-drop-before");
      if (drag.slot === count && i === count - 1)
        classes.push("sift-drop-after");
    }
    return classes.join(" ");
  };

  return { itemRef, handleProps, itemClass, dragging: drag?.active ?? false };
}
