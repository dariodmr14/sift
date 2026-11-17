// Piezas de la barra que no son la lista: texto resaltado, estado vacío y los paneles de
// un Espacio en marcha y de su resumen.

import {
  commands,
  type Segment,
  type SpaceProgress_Deserialize,
} from "../bindings";
import { progressText, type SummaryLine } from "./runSummary";
import { Glyph, type GlyphName } from "../components/Glyph";
import { Keycap } from "../components/Keycap";

export type SpaceProgress = SpaceProgress_Deserialize;

export const openSettings = (newSpace = false) =>
  void commands.openSettings(newSpace);

/**
 * Texto con lo que encaja marcado con la firma (`.sift-match`). Los trozos vienen de
 * Rust. `whole`: el texto entero es lo que encaja (un pasaje que llegó por significado,
 * sin tus palabras dentro).
 */
export function Highlighted({
  segments,
  whole = false,
}: {
  segments: Segment[];
  whole?: boolean;
}) {
  if (whole)
    return (
      <mark className="sift-match">{segments.map((s) => s.text).join("")}</mark>
    );
  return (
    <>
      {segments.map((s, i) =>
        s.hit ? (
          <mark key={i} className="sift-match">
            {s.text}
          </mark>
        ) : (
          <span key={i}>{s.text}</span>
        ),
      )}
    </>
  );
}

export function EmptyState() {
  return (
    <div className="flex h-full flex-col items-center justify-center gap-3 p-6 text-center">
      <p className="text-xl font-semibold">Aún no tienes Espacios</p>
      <p className="max-w-sm text-sm text-muted">
        Un Espacio abre de golpe todo lo de un contexto: carpetas, webs, apps y
        comandos.
      </p>
      <button
        type="button"
        onMouseDown={(e) => e.preventDefault()}
        onClick={() => openSettings(true)}
        className="mt-2 rounded-item bg-selected px-4 py-2 text-sm"
      >
        Crear tu primer Espacio <Keycap k="enter" />
      </button>
    </div>
  );
}

export function RunningPanel({
  title,
  progress,
  cancelling,
}: {
  title: string;
  progress: SpaceProgress | null;
  cancelling: boolean;
}) {
  return (
    <div role="status" aria-live="polite" className="h-full p-5">
      <p className="mb-1 text-sm font-semibold">Abriendo «{title}»</p>
      <p className="text-sm text-muted">
        {cancelling
          ? "Cancelando…"
          : progress
            ? progressText({ ...progress, waited: progress.waited ?? null })
            : "Empezando…"}
      </p>
    </div>
  );
}

const LINE_MARK: Record<
  SummaryLine["state"],
  { mark: GlyphName; cls: string }
> = {
  ok: { mark: "check", cls: "text-success" },
  warning: { mark: "warning", cls: "text-accent-ink" },
  failed: { mark: "close", cls: "text-danger" },
  cancelled: { mark: "close", cls: "text-muted" },
  notRun: { mark: "dash", cls: "text-muted" },
};

export function ResultPanel({
  title,
  headline,
  lines,
}: {
  title: string;
  headline: string;
  lines: SummaryLine[];
}) {
  return (
    <div role="alert" className="h-full overflow-y-auto p-5">
      <p className="mb-3 text-sm font-semibold">
        «{title}»: {headline}
      </p>
      <ul className="space-y-1.5 text-sm select-text">
        {lines.map((l, i) => (
          <li key={i} className="flex gap-2">
            <span
              className={`w-4 shrink-0 text-center ${LINE_MARK[l.state].cls}`}
            >
              <Glyph name={LINE_MARK[l.state].mark} />
            </span>
            <span className="min-w-0">
              <span className={l.state === "notRun" ? "text-muted" : ""}>
                {l.label}
                {l.state === "notRun" && " (no se ejecutó)"}
              </span>
              {l.detail && (
                <span
                  className={`block font-mono text-xs whitespace-pre-wrap ${
                    l.state === "failed" ? "text-danger" : "text-muted"
                  }`}
                >
                  {l.detail}
                </span>
              )}
            </span>
          </li>
        ))}
      </ul>
      <p className="mt-4 text-xs text-muted">
        Revisa el Espacio en Ajustes si hace falta.
      </p>
    </div>
  );
}
