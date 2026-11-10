import type { ReactNode } from "react";
import {
  DndContext,
  PointerSensor,
  closestCenter,
  useSensor,
  useSensors,
  type Announcements,
  type UniqueIdentifier,
} from "@dnd-kit/core";
import {
  restrictToParentElement,
  restrictToVerticalAxis,
} from "@dnd-kit/modifiers";
import {
  SortableContext,
  useSortable,
  verticalListSortingStrategy,
} from "@dnd-kit/sortable";
import { CSS } from "@dnd-kit/utilities";
import { prefersReducedMotion } from "../appearance";

/*
 * Reordenar arrastrando con dnd-kit, una librería probada (el efecto de Notion o Linear):
 * la tarjeta sigue al puntero y las demás se apartan con una transición corta, solo con
 * `transform` (no recalcula el diseño). Se usa con eventos de puntero, no con el drag &
 * drop de HTML5, que en Tauri para Windows intercepta la ventana para recibir archivos.
 *
 * El teclado sigue siendo Alt+↑/↓ en cada tarjeta (lo pone quien use la lista).
 */

/** Píxeles que hay que mover antes de que empiece el arrastre: un clic no arrastra. */
const ACTIVATION_DISTANCE = 4;

const position = (ids: UniqueIdentifier[], id: UniqueIdentifier) =>
  ids.indexOf(id) + 1;

/** Lo que oye un lector de pantalla, en español (dnd-kit trae los textos en inglés). */
function announcements(ids: UniqueIdentifier[]): Announcements {
  return {
    onDragStart: ({ active }) =>
      `Cogido el elemento ${position(ids, active.id)} de ${ids.length}.`,
    onDragOver: ({ over }) =>
      over ? `Sobre la posición ${position(ids, over.id)}.` : undefined,
    onDragEnd: ({ over }) =>
      over
        ? `Soltado en la posición ${position(ids, over.id)}.`
        : "Soltado fuera de la lista: no cambia nada.",
    onDragCancel: () => "Cancelado: no cambia nada.",
  };
}

/** Mientras se arrastra: cursor de agarre y nada seleccionable en toda la página. */
function setSorting(on: boolean) {
  document.documentElement.classList.toggle("sift-sorting", on);
  if (on) window.getSelection()?.removeAllRanges();
}

/**
 * Lista vertical reordenable. `ids` en el orden actual; `onMove(from, to)` al soltar en
 * otro sitio (índices). Los hijos usan `useSortableItem` con el mismo id.
 */
export function SortableList({
  ids,
  onMove,
  children,
}: {
  ids: UniqueIdentifier[];
  onMove: (from: number, to: number) => void;
  children: ReactNode;
}) {
  const sensors = useSensors(
    useSensor(PointerSensor, {
      activationConstraint: { distance: ACTIVATION_DISTANCE },
    }),
  );
  return (
    <DndContext
      sensors={sensors}
      collisionDetection={closestCenter}
      modifiers={[restrictToVerticalAxis, restrictToParentElement]}
      accessibility={{
        announcements: announcements(ids),
        screenReaderInstructions: {
          draggable:
            "Para reordenar con el teclado, usa Alt+Flecha arriba o Alt+Flecha abajo.",
        },
      }}
      onDragStart={() => setSorting(true)}
      onDragCancel={() => setSorting(false)}
      onDragEnd={({ active, over }) => {
        setSorting(false);
        if (!over || active.id === over.id) return;
        onMove(ids.indexOf(active.id), ids.indexOf(over.id));
      }}
    >
      <SortableContext items={ids} strategy={verticalListSortingStrategy}>
        {children}
      </SortableContext>
    </DndContext>
  );
}

/**
 * Para cada elemento de una `SortableList`: `itemProps` van en el elemento que se mueve,
 * `handleProps` en el asa que se coge. `dragging` es si es el que va en la mano.
 */
export function useSortableItem(id: UniqueIdentifier) {
  const {
    setNodeRef,
    setActivatorNodeRef,
    attributes,
    listeners,
    transform,
    transition,
    isDragging,
  } = useSortable({
    id,
    attributes: { roleDescription: "elemento reordenable" },
    // Con «reducir movimiento», las demás saltan a su sitio sin animarse.
    transition: prefersReducedMotion()
      ? null
      : { duration: 180, easing: "cubic-bezier(0, 0, 0, 1)" },
  });
  return {
    dragging: isDragging,
    itemProps: {
      ref: setNodeRef,
      // Solo traslación: con escala, las tarjetas de distinta altura se deformarían.
      style: { transform: CSS.Translate.toString(transform), transition },
    },
    handleProps: {
      ref: setActivatorNodeRef,
      ...listeners,
      // dnd-kit pone role/tabIndex/aria de "botón arrastrable"; el asa ya es un botón.
      "aria-roledescription": attributes["aria-roledescription"],
      "aria-describedby": attributes["aria-describedby"],
      "aria-pressed": attributes["aria-pressed"],
    },
  };
}
