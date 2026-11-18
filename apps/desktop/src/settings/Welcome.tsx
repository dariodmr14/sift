import { useEffect, useRef, useState } from "react";
import { commands } from "../bindings";
import { Glyph } from "../components/Glyph";
import { Shortcut } from "../components/Keycap";
import { Button, ErrorText } from "./ui";
import { useWelcome } from "./useWelcome";

/** Tiempo que se ve «Pulsa… para empezar» antes de abrir la barra. */
const FINISH_DELAY_MS = 1500;

function Step({
  n,
  title,
  children,
}: {
  n: number;
  title: string;
  children: React.ReactNode;
}) {
  return (
    <li className="flex gap-3">
      <span
        aria-hidden
        className="flex size-6 shrink-0 items-center justify-center rounded-full bg-hover text-xs"
      >
        {n}
      </span>
      <div className="min-w-0 flex-1 space-y-2">
        <h3 className="text-sm font-semibold">{title}</h3>
        {children}
      </div>
    </li>
  );
}

/**
 * Los tres pasos de la bienvenida. `first-run`: la primera vez, sin navegación y con
 * «Empezar». `settings`: la misma guía dentro de Ajustes, para volver a verla.
 */
export function Welcome({
  mode,
  onGoTo,
  onCreateSpace,
}: {
  mode: "first-run" | "settings";
  onGoTo?: (page: "general") => void;
  onCreateSpace?: () => void;
}) {
  const w = useWelcome();
  const firstRun = mode === "first-run";
  const [hintOpen, setHintOpen] = useState(false);
  const [finishing, setFinishing] = useState(false);
  const titleRef = useRef<HTMLHeadingElement>(null);

  useEffect(() => {
    if (firstRun) titleRef.current?.focus();
  }, [firstRun]);

  useEffect(() => {
    if (!finishing) return;
    const timer = setTimeout(
      () => void commands.finishWelcome(true),
      FINISH_DELAY_MS,
    );
    return () => clearTimeout(timer);
  }, [finishing]);

  const hotkey = w.view?.hotkey ?? "";

  if (finishing) {
    return (
      <div
        role="status"
        className="flex h-full items-center justify-center gap-2 text-xl"
      >
        <span>Pulsa</span>
        <Shortcut keys={hotkey} />
        <span>para empezar</span>
      </div>
    );
  }

  const smart = w.smart;
  let smartBody: React.ReactNode;
  if (smart?.ready) {
    smartBody = <p className="text-sm text-success">Lista</p>;
  } else if (
    smart &&
    smart.enabled &&
    !smart.awaitingPermission &&
    smart.downloadMb > 0
  ) {
    smartBody = (
      <p role="status" className="text-sm tabular-nums">
        Descargando {smart.downloadedMb} de {smart.downloadMb} MB
      </p>
    );
  } else if (w.requested) {
    smartBody = (
      <p role="status" className="text-sm">
        Preparando la descarga…
      </p>
    );
  } else if (w.skipped) {
    smartBody = (
      <p className="text-sm text-muted">
        Puedes activarla cuando quieras en Ajustes → General.
      </p>
    );
  } else {
    smartBody = (
      <div className="flex gap-2">
        <Button variant="primary" onClick={() => void w.allowDownload()}>
          Descargar ({w.downloadMb} MB)
        </Button>
        <Button variant="ghost" onClick={w.skip}>
          Ahora no
        </Button>
      </div>
    );
  }

  return (
    <div
      className={`h-full overflow-y-auto p-6 ${firstRun ? "flex justify-center" : ""}`}
    >
      <div className="w-full max-w-[560px] space-y-6">
        {firstRun ? (
          <header className="space-y-2">
            <h1
              ref={titleRef}
              tabIndex={-1}
              className="text-xl font-semibold outline-none"
            >
              Te damos la bienvenida a Sift
            </h1>
            <p className="text-sm text-muted">
              Sift busca en tus archivos por su nombre, por lo que dicen y por
              lo que tratan, y abre de una vez todo lo que usas para una tarea.
              Todo se queda en tu PC.
            </p>
          </header>
        ) : (
          <h2 className="text-sm font-semibold">Primeros pasos</h2>
        )}

        <ol className="space-y-6">
          <Step n={1} title="El atajo">
            <p className="flex flex-wrap items-center gap-2 text-sm">
              <span>Sift se abre con</span>
              <Shortcut keys={hotkey} />
              {/* Botón normal (con fondo), como «Añadir Documentos»: el fantasma, sin fondo,
                  se leía como texto. */}
              <Button
                onClick={() =>
                  firstRun ? setHintOpen((o) => !o) : onGoTo?.("general")
                }
              >
                Cambiar
              </Button>
            </p>
            {firstRun && hintOpen && (
              <p role="status" className="text-xs text-muted">
                Puedes cambiarlo luego en Ajustes → General.
              </p>
            )}
          </Step>

          <Step n={2} title="Dónde buscar">
            <p className="text-sm">
              Elige las carpetas con tus archivos. Puedes añadir más cuando
              quieras.
            </p>
            <div className="flex flex-wrap items-center gap-2">
              {w.view?.suggested.map((f) =>
                f.added ? (
                  <span
                    key={f.kind}
                    className="flex items-center gap-2 px-3 py-1.5 text-sm"
                  >
                    <Glyph name="check" className="text-success" />
                    {f.label} añadida
                  </span>
                ) : (
                  <Button
                    key={f.kind}
                    onClick={() => void w.addSuggested(f.kind)}
                  >
                    Añadir {f.label}
                  </Button>
                ),
              )}
              <Button onClick={() => void w.addOther()}>
                Elegir otra carpeta…
              </Button>
            </div>
            {w.error && <ErrorText>{w.error}</ErrorText>}
          </Step>

          <Step n={3} title="Búsqueda inteligente">
            <p className="text-sm">
              Encuentra también lo que trata de lo mismo con otras palabras.
              Necesita descargar una vez unos {w.downloadMb} MB; después
              funciona sin conexión y nada sale de tu PC.
            </p>
            {smartBody}
            {smart?.note && <p className="text-xs text-muted">{smart.note}</p>}
          </Step>
        </ol>

        {firstRun && (
          <footer className="flex gap-2">
            <Button
              onClick={() => {
                void commands
                  .finishWelcome(false)
                  .then(() => onCreateSpace?.());
              }}
            >
              Crear tu primer Espacio
            </Button>
            <Button variant="primary" onClick={() => setFinishing(true)}>
              Empezar
            </Button>
          </footer>
        )}
      </div>
    </div>
  );
}
