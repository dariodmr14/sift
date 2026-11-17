// La lista de resultados de la barra: filas seleccionables con cabeceras de sección.

import { Fragment, useEffect, useMemo, useRef } from "react";
import { Shortcut } from "../components/Keycap";
import { SECTION_LABEL, sectionStarts } from "./files";
import type { Item } from "./items";

export function ResultsList({
  items,
  selected,
  iconsHidden,
  onHover,
  onActivate,
}: {
  items: Item[];
  selected: number;
  iconsHidden: boolean;
  onHover: (index: number) => void;
  onActivate: (item: Item) => void;
}) {
  const listRef = useRef<HTMLUListElement>(null);
  const starts = useMemo(
    () => sectionStarts(items.map((i) => i.section)),
    [items],
  );

  // Que el elemento seleccionado siempre quede a la vista al moverse con las flechas.
  useEffect(() => {
    listRef.current
      ?.querySelector(`[data-index="${selected}"]`)
      ?.scrollIntoView({ block: "nearest" });
  }, [selected]);

  return (
    <ul
      id="results"
      ref={listRef}
      role="listbox"
      aria-label="Resultados"
      className="h-full overflow-y-auto p-2"
    >
      {items.map((item, i) => (
        <Fragment key={item.key}>
          {starts.has(i) && (
            // Cabecera de sección: no se puede seleccionar (no es una opción).
            <li
              role="presentation"
              className="px-3 pt-2 pb-1 text-xs font-semibold text-muted first:pt-0"
            >
              {SECTION_LABEL[item.section]}
            </li>
          )}
          <li
            id={`result-${item.key}`}
            data-index={i}
            role="option"
            aria-selected={i === selected}
            onMouseMove={() => onHover(i)}
            onMouseDown={(e) => e.preventDefault()} // que el input no pierda el foco
            onClick={() => onActivate(item)}
            className="sift-option flex items-center gap-3 px-3 py-2"
          >
            {!iconsHidden && (
              <span
                className="flex w-7 shrink-0 items-center justify-center"
                aria-hidden
              >
                {item.icon}
              </span>
            )}
            <span className="min-w-0 flex-1">
              <span className="block truncate pb-0.5 text-sm">
                {item.title}
              </span>
              <span className="block truncate pb-0.5 text-xs text-muted">
                {item.subtitle}
              </span>
            </span>
            {i === selected && (
              <span aria-hidden className="shrink-0">
                <Shortcut keys={item.shortcut ?? "enter"} />
              </span>
            )}
          </li>
        </Fragment>
      ))}
    </ul>
  );
}
