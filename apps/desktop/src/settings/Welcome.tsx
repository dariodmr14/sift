// Ajustes → Primeros pasos: lo básico en una página (el atajo, dónde buscar y la búsqueda
// inteligente) y el botón para repetir la introducción guiada. La primera vez se ve la
// introducción (`Intro.tsx`), no esta página.

import { Glyph } from "../components/Glyph";
import { Shortcut } from "../components/Keycap";
import { hotkeyText } from "./introSteps";
import { Button, ErrorText } from "./ui";
import { useWelcome } from "./useWelcome";

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

export function Welcome({
  onGoTo,
  onReplay,
}: {
  onGoTo?: (page: "general") => void;
  /** «Ver la introducción otra vez». */
  onReplay?: () => void;
}) {
  const w = useWelcome();
  const hotkey = w.view?.hotkey ?? "";
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
    <div className="h-full overflow-y-auto p-6">
      <div className="w-full max-w-[560px] space-y-6">
        <header className="flex flex-wrap items-center justify-between gap-3">
          <h2 className="text-sm font-semibold">Primeros pasos</h2>
          <Button onClick={onReplay}>Ver la introducción otra vez</Button>
        </header>

        <ol className="space-y-6">
          <Step n={1} title="El atajo">
            <p className="flex flex-wrap items-center gap-2 text-sm">
              <span>Sift se abre con</span>
              {hotkey && <Shortcut keys={hotkeyText(hotkey)} />}
              {/* Botón normal (con fondo), como «Añadir Documentos»: el fantasma, sin fondo,
                  se leía como texto. */}
              <Button onClick={() => onGoTo?.("general")}>Cambiar</Button>
            </p>
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
      </div>
    </div>
  );
}
