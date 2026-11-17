// La vista previa de un resultado, en el hueco de la lista: qué archivo es, por qué ha
// salido y un fragmento más largo que el de la lista, con lo que encaja marcado.

import type { ReactNode } from "react";
import type { FileHit } from "../bindings";
import { shortFolder } from "./files";
import type { Item } from "./items";
import { Highlighted } from "./panels";
import { previewReason, reasonLabel } from "./preview";
import type { PreviewState } from "./usePreview";

export const NO_PREVIEW_TEXT = "Este resultado no tiene vista previa.";

/** El nombre que se dice al lector de pantalla y se enseña si no hay archivo. */
export function previewName(item: Item, hit?: FileHit): string {
  if (hit) return hit.name;
  return typeof item.title === "string" ? item.title : "";
}

export function PreviewPanel({
  item,
  hit,
  state,
  icon,
}: {
  item: Item;
  hit?: FileHit;
  state: PreviewState | null;
  /** El icono del archivo; `undefined` con los iconos ocultos en Ajustes. */
  icon?: ReactNode;
}) {
  const name = previewName(item, hit);
  const reason = hit ? previewReason(hit) : null;
  const preview = state?.status === "ready" ? state.preview : null;
  const page = preview?.page ?? hit?.page ?? null;

  return (
    <section
      role="region"
      aria-label={`Vista previa de ${name}`}
      className="flex h-full min-h-0 flex-col"
    >
      <header className="flex items-center gap-3 border-b border-line px-5 py-3">
        {icon && (
          <span
            className="flex w-7 shrink-0 items-center justify-center"
            aria-hidden
          >
            {icon}
          </span>
        )}
        <div className="min-w-0 flex-1">
          <p className="truncate text-sm font-semibold">
            {hit ? <Highlighted segments={hit.nameSegments} /> : name}
          </p>
          {hit && reason && (
            <p className="truncate text-xs text-muted">
              {shortFolder(hit.path)}
            </p>
          )}
          {hit && reason && (
            <p className="truncate text-xs text-muted">
              <span
                title={
                  reason.kind === "meaning"
                    ? "No contiene tus palabras: Sift lo ha encontrado por su significado."
                    : undefined
                }
              >
                {reasonLabel(reason)}
              </span>
              {page != null && <> · pág. {page}</>}
            </p>
          )}
        </div>
      </header>
      <div className="min-h-0 flex-1 overflow-y-auto px-5 py-4">
        {!hit ? (
          <p className="text-sm text-muted">{NO_PREVIEW_TEXT}</p>
        ) : state?.status === "error" ? (
          <p className="text-sm text-muted">{state.message}</p>
        ) : preview ? (
          preview.text.length === 0 ? (
            <p className="text-sm text-muted">
              Este archivo no tiene texto que enseñar.
            </p>
          ) : (
            <p
              className={
                preview.monospace
                  ? "font-mono text-xs leading-relaxed whitespace-pre-wrap"
                  : "text-sm leading-relaxed"
              }
            >
              <Highlighted
                segments={preview.text}
                whole={hit.byMeaning && !preview.text.some((s) => s.hit)}
              />
            </p>
          )
        ) : null}
      </div>
    </section>
  );
}
