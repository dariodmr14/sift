// La vista previa de un resultado, en el hueco de la lista. De un archivo: qué es, por qué
// ha salido y un fragmento más largo que el de la lista, con lo que encaja marcado. De un
// Espacio: sus acciones en orden (qué abrirá, qué esperará y dónde pondrá cada ventana).

import type { ReactNode } from "react";
import type { FileHit } from "../bindings";
import { Glyph } from "../components/Glyph";
import { shortFolder } from "./files";
import type { Item } from "./items";
import { Highlighted } from "./panels";
import { previewReason, reasonLabel } from "./preview";
import { spaceSteps } from "./spacePreview";
import type { PreviewState } from "./usePreview";

export const NO_PREVIEW_TEXT = "Este resultado no tiene vista previa.";

/** El nombre que se dice al lector de pantalla y se enseña si no hay archivo. */
export function previewName(item: Item, hit?: FileHit): string {
  if (hit) return hit.name;
  if (item.action.kind === "space") return item.action.view.space.name;
  return typeof item.title === "string" ? item.title : "";
}

/** Si un resultado tiene vista previa: los archivos encontrados y los Espacios. */
export function canPreview(item: Item | undefined): boolean {
  return !!item && (!!item.hit || item.action.kind === "space");
}

export function PreviewPanel({
  item,
  hit,
  state,
  icon,
  onBack,
}: {
  item: Item;
  hit?: FileHit;
  state: PreviewState | null;
  /** El icono del archivo; `undefined` con los iconos ocultos en Ajustes. */
  icon?: ReactNode;
  /** Vuelve a la lista (el botón ←, para quien use el ratón; con teclado, Esc). */
  onBack: () => void;
}) {
  const name = previewName(item, hit);
  const space = item.action.kind === "space" ? item.action.view.space : null;
  const reason = hit ? previewReason(hit) : null;
  const preview = state?.status === "ready" ? state.preview : null;
  const page = preview?.page ?? hit?.page ?? null;

  return (
    <section
      role="region"
      aria-label={`Vista previa de ${name}`}
      className="flex h-full min-h-0 flex-col"
    >
      <header className="flex items-center gap-3 border-b border-line py-3 pr-5 pl-3">
        <button
          type="button"
          tabIndex={-1}
          onMouseDown={(e) => e.preventDefault()} // que la caja no pierda el foco
          onClick={onBack}
          title="Volver a la lista (Esc)"
          aria-label="Volver a la lista"
          className="flex size-8 shrink-0 items-center justify-center rounded-item text-muted hover:bg-hover hover:text-fg"
        >
          <Glyph name="back" />
        </button>
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
          {space && (
            <p className="truncate text-xs text-muted">
              {space.actions.length === 1
                ? "Espacio · 1 acción"
                : `Espacio · ${space.actions.length} acciones`}
            </p>
          )}
        </div>
      </header>
      <div className="min-h-0 flex-1 overflow-y-auto px-5 py-4">
        {space ? (
          <SpaceSteps actions={space.actions} />
        ) : !hit ? (
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
              data-selectable
              className={
                preview.monospace
                  ? "font-mono text-xs leading-relaxed whitespace-pre-wrap"
                  : "text-sm leading-relaxed whitespace-pre-line"
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

/** Las acciones de un Espacio, numeradas en el orden en que se ejecutan. */
function SpaceSteps({
  actions,
}: {
  actions: Parameters<typeof spaceSteps>[0];
}) {
  if (actions.length === 0)
    return (
      <p className="text-sm text-muted">
        Este Espacio aún no tiene acciones. Añádelas en Ajustes.
      </p>
    );
  return (
    <ol aria-label="Acciones, en orden" className="space-y-3" data-selectable>
      {spaceSteps(actions).map((s, i) => (
        <li key={i} className="flex gap-3">
          <span
            aria-hidden
            className="flex size-6 shrink-0 items-center justify-center rounded-full bg-selected text-xs font-semibold"
          >
            {i + 1}
          </span>
          <span className="min-w-0 pt-0.5">
            <span className="block text-sm break-words">{s.title}</span>
            {s.details.map((d, j) => (
              <span key={j} className="block text-xs break-words text-muted">
                {d}
              </span>
            ))}
          </span>
        </li>
      ))}
    </ol>
  );
}
