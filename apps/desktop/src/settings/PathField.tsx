// Campo de ruta con sugerencias del índice de Sift (Fase 6) y el selector de Windows.
//
// Mientras escribes, debajo salen archivos o carpetas del índice que encajan por nombre
// (ver `sift_search::paths`): ↑/↓ para moverse, Enter para elegir, Esc para cerrar.
// El botón «Elegir…» sigue abriendo el diálogo de Windows, para lo que no está indexado.

import { useEffect, useId, useRef, useState } from "react";
import { commands, type PathKind } from "../bindings";
import { moveSelection } from "../launcher/selection";
import { shouldSuggest } from "./pathSuggest";
import { Button, TextInput } from "./ui";

/** Espera tras la última tecla antes de preguntar (no una consulta por letra). */
const DEBOUNCE_MS = 120;

export function PathField({
  kind,
  value,
  onChange,
  placeholder,
  label,
}: {
  kind: PathKind;
  value: string;
  onChange: (path: string) => void;
  placeholder: string;
  label: string;
}) {
  const [suggestions, setSuggestions] = useState<string[]>([]);
  const [active, setActive] = useState(-1);
  // Solo se sugiere mientras escribes: no al abrir el editor ni tras elegir una.
  const [typing, setTyping] = useState(false);
  const seq = useRef(0);
  const listId = useId();

  useEffect(() => {
    const mine = ++seq.current;
    if (!typing || !shouldSuggest(value)) {
      // Se limpia en la siguiente vuelta (no dentro del efecto).
      void Promise.resolve().then(() => {
        if (mine === seq.current) setSuggestions([]);
      });
      return;
    }
    const timer = setTimeout(() => {
      void commands.suggestPaths(value, kind).then((r) => {
        // Si ya se pidió otra cosa, esta respuesta no vale.
        if (mine !== seq.current || r.status !== "ok") return;
        setSuggestions(r.data);
        setActive(-1);
      });
    }, DEBOUNCE_MS);
    return () => clearTimeout(timer);
  }, [value, kind, typing]);

  const open = suggestions.length > 0;

  function choose(path: string) {
    setTyping(false);
    setSuggestions([]);
    onChange(path);
  }

  async function pick() {
    const r = await commands.pickPath(kind);
    if (r.status === "ok" && r.data) choose(r.data);
  }

  return (
    <div className="flex gap-2">
      <div className="relative flex-1">
        <TextInput
          aria-label={label}
          role="combobox"
          aria-expanded={open}
          aria-controls={listId}
          aria-autocomplete="list"
          aria-activedescendant={
            open && active >= 0 ? `${listId}-${active}` : undefined
          }
          placeholder={placeholder}
          value={value}
          onChange={(e) => {
            setTyping(true);
            onChange(e.currentTarget.value);
          }}
          onBlur={() => setSuggestions([])}
          onKeyDown={(e) => {
            if (!open) return;
            if (e.key === "ArrowDown" || e.key === "ArrowUp") {
              e.preventDefault();
              setActive((i) =>
                moveSelection(
                  i,
                  e.key === "ArrowDown" ? 1 : -1,
                  suggestions.length,
                ),
              );
            } else if (e.key === "Enter" && active >= 0) {
              e.preventDefault();
              choose(suggestions[active]);
            } else if (e.key === "Escape") {
              // Cierra la lista, no el editor.
              e.stopPropagation();
              setSuggestions([]);
            }
          }}
        />
        {open && (
          <ul
            id={listId}
            role="listbox"
            aria-label="Sugerencias de tus carpetas indexadas"
            className="absolute inset-x-0 top-full z-20 mt-1 max-h-72 overflow-y-auto p-1 sift-glass"
          >
            {suggestions.map((path, i) => (
              <li
                key={path}
                id={`${listId}-${i}`}
                role="option"
                aria-selected={i === active}
                // Que el campo no pierda el foco (y la lista no se cierre) al hacer clic.
                onMouseDown={(e) => e.preventDefault()}
                onMouseMove={() => setActive(i)}
                onClick={() => choose(path)}
                className="sift-option cursor-default px-3 py-1.5"
              >
                <span className="block truncate text-sm">
                  {path.split(/[\\/]/).pop()}
                </span>
                <span className="block truncate text-xs text-muted">
                  {path}
                </span>
              </li>
            ))}
          </ul>
        )}
      </div>
      <Button onClick={() => void pick()}>Elegir…</Button>
    </div>
  );
}
