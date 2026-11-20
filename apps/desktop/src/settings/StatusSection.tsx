// Ajustes → Estado: tarjetas que dicen de un vistazo si todo va bien (el atajo, las
// carpetas, la búsqueda inteligente y tus datos). Lo técnico va en «Avanzado», plegado.

import { useCallback, useEffect, useState } from "react";
import {
  commands,
  events,
  type IndexStatus,
  type LatencySummary,
  type Status,
} from "../bindings";
import { Glyph, type GlyphName } from "../components/Glyph";
import { count } from "../format";
import { statusCards, type Tone } from "./statusCards";
import { SiftLoader } from "../components/SiftLoader";

const BACKEND_LABEL = {
  registerHotKey:
    "RegisterHotKey: el atajo está registrado en Windows, que avisa a Sift al pulsarlo (lo normal).",
  keyboardHook:
    "Hook de teclado: Windows no dejó registrar el atajo (quizá otro programa lo usa), así que Sift escucha el teclado para detectarlo.",
} as const;

/** El icono y su color por tono. El texto ya dice lo mismo: el color nunca va solo. */
const TONE: Record<Tone, { glyph: GlyphName; cls: string; label: string }> = {
  ok: { glyph: "check", cls: "text-success", label: "Bien" },
  working: { glyph: "sync", cls: "text-accent-ink", label: "En marcha" },
  warning: { glyph: "warning", cls: "text-accent-ink", label: "Aviso" },
  error: { glyph: "warning", cls: "text-danger", label: "Problema" },
  off: { glyph: "dash", cls: "text-muted", label: "Apagado" },
};

function latencyText(l: LatencySummary | null | undefined, goal: string) {
  if (!l || l.samples === 0) return "Aún sin medir.";
  const last = l.lastMs != null ? `${l.lastMs} ms la última vez. ` : "";
  return `${last}p50 ${l.p50Ms ?? "—"} ms y p95 ${l.p95Ms ?? "—"} ms en las últimas ${count(l.samples)} veces: la mitad tardó menos de ${l.p50Ms ?? "—"} ms y 19 de cada 20, menos de ${l.p95Ms ?? "—"} ms. Objetivo: ${goal}.`;
}

export function StatusSection({ status }: { status: Status }) {
  const [index, setIndex] = useState<IndexStatus | null>(null);
  const [roots, setRoots] = useState<{ folders: number; files: number }>({
    folders: 0,
    files: 0,
  });

  const reloadRoots = useCallback(
    () =>
      commands.listRoots().then((r) => {
        if (r.status === "ok")
          setRoots({
            folders: r.data.length,
            files: r.data.reduce((sum, root) => sum + root.files, 0),
          });
      }),
    [],
  );

  useEffect(() => {
    void reloadRoots();
    void commands.indexStatus().then(setIndex);
    const unlisten = events.indexProgress.listen((e) => {
      setIndex(e.payload);
      if (!e.payload.running) void reloadRoots();
    });
    return () => {
      void unlisten.then((stop) => stop());
    };
  }, [reloadRoots]);

  const cards = statusCards(status, index, roots);
  const allGood = cards.every((c) => c.tone === "ok" || c.tone === "off");

  return (
    <div className="space-y-6">
      <p className="text-sm text-muted">
        {allGood
          ? "Todo va bien."
          : cards.some((c) => c.tone === "error")
            ? "Algo no funciona: mira lo marcado."
            : "Sift está trabajando o tiene algún aviso."}
      </p>
      <ul className="grid grid-cols-1 gap-3 sm:grid-cols-2">
        {cards.map((c) => (
          <li
            key={c.id}
            className="flex gap-3 rounded-item border border-line bg-hover/40 p-4"
          >
            <span
              role="img"
              aria-label={TONE[c.tone].label}
              className={`flex size-6 shrink-0 items-center justify-center ${TONE[c.tone].cls}`}
            >
              {c.tone === "working" ? (
                <SiftLoader />
              ) : (
                <Glyph name={TONE[c.tone].glyph} />
              )}
            </span>
            <span className="min-w-0">
              <span className="block text-sm font-semibold">{c.title}</span>
              <span className="block text-xs break-words text-muted">
                {c.detail}
              </span>
            </span>
          </li>
        ))}
      </ul>
      <details className="group space-y-3">
        <summary className="flex cursor-pointer list-none items-center gap-2 rounded-item text-sm font-semibold outline-none select-none focus-visible:ring-2 focus-visible:ring-accent-ink">
          <span
            aria-hidden
            className="inline-block transition-transform duration-150 group-open:rotate-90 motion-reduce:transition-none"
          >
            <Glyph name="chevronRight" className="text-xs" />
          </span>
          Avanzado
        </summary>
        <dl className="grid grid-cols-[auto_1fr] gap-x-6 gap-y-2 pt-3 text-sm">
          <dt className="text-muted">Atajo</dt>
          <dd>{status.hotkey ?? "Ninguno"}</dd>
          <dt className="text-muted">Cómo se escucha el atajo</dt>
          <dd>{status.backend ? BACKEND_LABEL[status.backend] : "—"}</dd>
          <dt className="text-muted">Material de la barra</dt>
          <dd>
            {status.acrylic
              ? "Acrylic: el desenfoque translúcido de Windows."
              : "Opaco: Windows no ofrece Acrylic (el desenfoque translúcido) aquí."}
          </dd>
          <dt className="text-muted">Tiempo de apertura</dt>
          <dd>
            {latencyText(status.latency, "menos de 100 ms")} Se mide desde que
            pulsas el atajo hasta que la barra está en pantalla.
          </dd>
          <dt className="text-muted">Tiempo de búsqueda</dt>
          <dd>{latencyText(status.searchLatency, "menos de 50 ms")}</dd>
          <dt className="text-muted">Vigilancia de carpetas</dt>
          <dd>
            {!index
              ? "—"
              : index.watching
                ? "En tiempo real: los cambios se ven al momento."
                : "Sin vigilancia en tiempo real: los cambios se ven al volver a abrir Sift."}
          </dd>
          <dt className="text-muted">Lector de PDF</dt>
          <dd>
            {!index
              ? "—"
              : index.readsPdf
                ? "PDFium, listo."
                : "No está: los PDF se encuentran solo por su nombre. Reinstalar Sift lo arregla."}
          </dd>
        </dl>
      </details>
    </div>
  );
}
