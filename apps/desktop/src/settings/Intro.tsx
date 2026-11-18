// La introducción guiada: cinco pasos, uno por pantalla, que enseñan haciendo (pulsar el
// atajo, elegir carpetas, buscar de verdad, la búsqueda inteligente y el primer Espacio).
// La primera vez ocupa la ventana de Ajustes; después se puede repetir desde «Primeros
// pasos». Cada paso entra deslizándose 16 px (nada con «reducir movimiento»).

import { useEffect, useRef, useState } from "react";
import { commands, events, type FileHit, type RootView } from "../bindings";
import { Glyph } from "../components/Glyph";
import { Keycap, Shortcut } from "../components/Keycap";
import { shortFolder } from "../launcher/files";
import { Highlighted } from "../launcher/panels";
import { useFileSearch } from "../launcher/useFileSearch";
import { folderName, shortParent, statusLine } from "./folders";
import {
  clampStep,
  firstSpaceDraft,
  hotkeyText,
  INTRO_STEPS,
  stepLabel,
  type Direction,
} from "./introSteps";
import { Button, ErrorText, Field, TextInput } from "./ui";
import { useWelcome } from "./useWelcome";

/** Tiempo que se ve «Pulsa… para empezar» antes de abrir la barra. */
const FINISH_DELAY_MS = 1500;

export function Intro({
  mode,
  onClose,
}: {
  /** `first-run`: la primera vez. `replay`: repetida desde Ajustes. */
  mode: "first-run" | "replay";
  /** Al saltarla o terminarla sin abrir la barra: vuelve a Ajustes. */
  onClose: () => void;
}) {
  const w = useWelcome();
  const [step, setStep] = useState(0);
  const [direction, setDirection] = useState<Direction>("forward");
  const [finishing, setFinishing] = useState(false);
  const titleRef = useRef<HTMLHeadingElement>(null);
  const firstRender = useRef(true);

  // El foco va al título de cada paso: el lector de pantalla lee «Paso 2 de 5: …».
  useEffect(() => {
    if (firstRender.current) {
      firstRender.current = false;
      if (mode === "replay") return;
    }
    titleRef.current?.focus();
  }, [step, mode]);

  useEffect(() => {
    if (!finishing) return;
    const timer = setTimeout(
      () => void commands.finishWelcome(true),
      FINISH_DELAY_MS,
    );
    return () => clearTimeout(timer);
  }, [finishing]);

  const go = (to: number) => {
    setDirection(to < step ? "back" : "forward");
    setStep(clampStep(to));
  };
  const skip = () => {
    // Saltar no pregunta: se puede repetir desde Ajustes → Primeros pasos.
    if (mode === "first-run")
      void commands.finishWelcome(false).then(() => onClose());
    else onClose();
  };
  const finish = () => {
    if (mode === "first-run") setFinishing(true);
    else onClose();
  };

  const hotkey = w.view?.hotkey ?? "Alt+Space";

  if (finishing) {
    return (
      <div
        role="status"
        className="flex h-full items-center justify-center gap-2 text-xl"
      >
        <span>Pulsa</span>
        <Shortcut keys={hotkeyText(hotkey)} />
        <span>para empezar</span>
      </div>
    );
  }

  const last = step === INTRO_STEPS.length - 1;
  const id = INTRO_STEPS[step].id;

  return (
    <div className="flex h-full justify-center overflow-y-auto p-6">
      <div className="flex w-full max-w-[560px] flex-col gap-6">
        <Progress step={step} />
        <section
          key={step}
          aria-labelledby="intro-title"
          className={`min-h-0 flex-1 space-y-4 ${
            direction === "forward" ? "sift-step-next" : "sift-step-back"
          }`}
        >
          <p className="text-xs text-muted">
            Paso {step + 1} de {INTRO_STEPS.length}
          </p>
          <h1
            id="intro-title"
            ref={titleRef}
            tabIndex={-1}
            aria-label={stepLabel(step)}
            className="text-xl font-semibold outline-none"
          >
            {step === 0 && mode === "first-run"
              ? "Te damos la bienvenida a Sift"
              : INTRO_STEPS[step].title}
          </h1>
          {id === "hotkey" && <HotkeyStep hotkey={hotkey} />}
          {id === "folders" && <FoldersStep w={w} />}
          {id === "search" && <SearchStep />}
          {id === "smart" && <SmartStep w={w} />}
          {id === "space" && <SpaceStep hotkey={hotkey} />}
        </section>
        <footer className="flex items-center gap-2 border-t border-line pt-4">
          <Button variant="ghost" onClick={skip}>
            Saltar la introducción
          </Button>
          <span className="flex-1" />
          {step > 0 && <Button onClick={() => go(step - 1)}>Atrás</Button>}
          <Button
            variant="primary"
            onClick={() => (last ? finish() : go(step + 1))}
          >
            {last
              ? mode === "first-run"
                ? "Empezar"
                : "Terminar"
              : "Siguiente"}
          </Button>
        </footer>
      </div>
    </div>
  );
}

/** Cinco segmentos finos: los hechos, llenos; el actual se llena con el acento. */
function Progress({ step }: { step: number }) {
  return (
    <div
      role="progressbar"
      aria-label="Progreso de la introducción"
      aria-valuemin={1}
      aria-valuemax={INTRO_STEPS.length}
      aria-valuenow={step + 1}
      aria-valuetext={stepLabel(step)}
      className="flex gap-1.5"
    >
      {INTRO_STEPS.map((s, i) => (
        <span
          key={s.id}
          className="h-1 flex-1 overflow-hidden rounded-full bg-hover"
        >
          <span
            className={`block h-full origin-left rounded-full bg-accent ${
              i < step ? "" : i === step ? "sift-fill" : "scale-x-0"
            }`}
          />
        </span>
      ))}
    </div>
  );
}

/** Un logro: ✓ verde que aparece con un poco de escala (sin rebotes). */
function Done({ children }: { children: React.ReactNode }) {
  return (
    <p role="status" className="sift-pop flex items-center gap-2 text-sm">
      <Glyph name="check" className="text-success" />
      {children}
    </p>
  );
}

/** Paso 1: pulsar el atajo de verdad. Rust avisa cuando la barra se abre. */
function HotkeyStep({ hotkey }: { hotkey: string }) {
  const [pressed, setPressed] = useState(false);
  const [hint, setHint] = useState(false);
  useEffect(() => {
    const unlisten = events.launcherShown.listen(() => setPressed(true));
    return () => {
      void unlisten.then((stop) => stop());
    };
  }, []);
  const keys = hotkeyText(hotkey).split("+");
  return (
    <div className="space-y-4">
      <p className="text-sm text-muted">
        Sift busca en tus archivos por su nombre, por lo que dicen y por lo que
        tratan, y abre de una vez todo lo que usas para una tarea. Todo se queda
        en tu PC.
      </p>
      <p className="text-sm">
        Se abre desde cualquier sitio con un atajo. Pruébalo: pulsa ahora
      </p>
      <p
        className={`flex items-center gap-1 text-xl ${pressed ? "sift-pressed" : ""}`}
        aria-label={hotkeyText(hotkey)}
      >
        {keys.map((k, i) => (
          <Keycap key={i} k={k} />
        ))}
      </p>
      {pressed ? (
        <Done>Eso es. Pulsa Esc para volver aquí y sigue.</Done>
      ) : (
        <p className="text-xs text-muted">
          Se abrirá la barra de Sift encima de esta ventana. Si no puedes
          pulsarlo ahora, sigue: puedes probarlo luego.
        </p>
      )}
      <div className="space-y-2">
        <Button onClick={() => setHint((o) => !o)}>Cambiar el atajo</Button>
        {hint && (
          <p role="status" className="text-xs text-muted">
            Cámbialo cuando quieras en Ajustes → General.
          </p>
        )}
      </div>
    </div>
  );
}

/** Paso 2: las carpetas sugeridas u otra, y cómo va su lectura. */
function FoldersStep({ w }: { w: ReturnType<typeof useWelcome> }) {
  const roots = useRoots(w.status?.running);
  const line = statusLine(
    w.status,
    roots.reduce((sum, r) => sum + r.files, 0),
    roots.length,
  );
  return (
    <div className="space-y-4">
      <p className="text-sm">
        Elige las carpetas con tus archivos. Sift las lee en segundo plano y las
        tiene al día. Puedes añadir más cuando quieras.
      </p>
      <div className="flex flex-wrap items-center gap-2">
        {w.view?.suggested.map((f) =>
          f.added ? (
            <span
              key={f.kind}
              className="sift-pop flex items-center gap-2 px-3 py-1.5 text-sm"
            >
              <Glyph name="check" className="text-success" />
              {f.label} añadida
            </span>
          ) : (
            <Button key={f.kind} onClick={() => void w.addSuggested(f.kind)}>
              Añadir {f.label}
            </Button>
          ),
        )}
        <Button onClick={() => void w.addOther()}>Elegir otra carpeta…</Button>
      </div>
      {roots.length > 0 && (
        <ul className="space-y-1 text-sm">
          {roots.map((r) => (
            <li key={r.id} className="flex items-center gap-2">
              <Glyph name="check" className="text-success" />
              <span className="truncate" title={r.path}>
                {folderName(r.path)}{" "}
                <span className="text-muted">· {shortParent(r.path)}</span>
              </span>
            </li>
          ))}
        </ul>
      )}
      <p role="status" className="text-xs text-muted">
        {line.text}
      </p>
      {w.error && <ErrorText>{w.error}</ErrorText>}
    </div>
  );
}

/** Las carpetas añadidas; se vuelven a pedir cuando la lectura termina. */
function useRoots(running: boolean | undefined) {
  const [roots, setRoots] = useState<RootView[]>([]);
  useEffect(() => {
    void commands.listRoots().then((r) => {
      if (r.status === "ok") setRoots(r.data);
    });
  }, [running]);
  return roots;
}

/** Paso 3: una caja como la de la barra, con resultados de verdad. */
function SearchStep() {
  const [query, setQuery] = useState("");
  const hits = useFileSearch(query, null).slice(0, 5);
  return (
    <div className="space-y-4">
      <p className="text-sm">
        Escribe una palabra que esté en alguno de tus archivos, o de qué trata.
        Es la misma búsqueda que la de la barra.
      </p>
      <TextInput
        aria-label="Buscar"
        placeholder="Buscar"
        value={query}
        onChange={(e) => setQuery(e.currentTarget.value)}
        className="h-10 text-xl"
      />
      {query.trim() && hits.length === 0 ? (
        <p className="text-sm text-muted">
          Aún nada. Si Sift sigue leyendo tus carpetas, saldrá en cuanto
          termine; puedes seguir.
        </p>
      ) : (
        <ul aria-label="Resultados" className="space-y-1">
          {hits.map((h: FileHit) => (
            <li key={h.fileId} className="sift-swap">
              <button
                type="button"
                onClick={() => void commands.openFile(h.path, h.page ?? null)}
                className="w-full rounded-item px-3 py-2 text-left outline-none hover:bg-hover focus-visible:ring-2 focus-visible:ring-accent-ink"
              >
                <span className="block truncate text-sm">
                  <Highlighted segments={h.nameSegments} />
                </span>
                <span className="block truncate text-xs text-muted">
                  {h.snippet.length ? (
                    <Highlighted
                      segments={h.snippet}
                      whole={h.byMeaning && !h.snippet.some((s) => s.hit)}
                    />
                  ) : (
                    shortFolder(h.path)
                  )}
                </span>
              </button>
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}

/** Paso 4: qué es la búsqueda inteligente y el permiso para descargarla. */
function SmartStep({ w }: { w: ReturnType<typeof useWelcome> }) {
  const smart = w.smart;
  return (
    <div className="space-y-4">
      <p className="text-sm">
        Encuentra también lo que trata de lo mismo con otras palabras: buscas
        «gastos del piso» y sale tu hoja «alquiler y facturas».
      </p>
      <p className="text-sm text-muted">
        Necesita descargar una vez unos {w.downloadMb} MB. Después funciona sin
        conexión y nada sale de tu PC.
      </p>
      {smart?.ready ? (
        <Done>Lista</Done>
      ) : smart &&
        smart.enabled &&
        !smart.awaitingPermission &&
        smart.downloadMb > 0 ? (
        <p role="status" className="text-sm tabular-nums">
          Descargando {smart.downloadedMb} de {smart.downloadMb} MB
        </p>
      ) : w.requested ? (
        <p role="status" className="text-sm">
          Preparando la descarga…
        </p>
      ) : w.skipped ? (
        <p className="text-sm text-muted">
          Puedes activarla cuando quieras en Ajustes → General.
        </p>
      ) : (
        <div className="flex gap-2">
          <Button variant="primary" onClick={() => void w.allowDownload()}>
            Descargar ({w.downloadMb} MB)
          </Button>
          <Button variant="ghost" onClick={w.skip}>
            Ahora no
          </Button>
        </div>
      )}
      {smart?.note && <p className="text-xs text-muted">{smart.note}</p>}
    </div>
  );
}

/** Paso 5: un Espacio sencillo (nombre, carpetas y una web), o seguir sin él. */
function SpaceStep({ hotkey }: { hotkey: string }) {
  const roots = useRoots(undefined);
  const [name, setName] = useState("");
  const [picked, setPicked] = useState<string[] | null>(null);
  const [url, setUrl] = useState("");
  const [created, setCreated] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  // Por defecto, todas las carpetas añadidas marcadas.
  const folders = picked ?? roots.map((r) => r.path);
  const draft = firstSpaceDraft(name, folders, url);

  async function create() {
    if (!draft) return;
    const r = await commands.createSpace(draft);
    if (r.status === "ok") {
      setError(null);
      setCreated(r.data.name);
    } else setError(r.error.message);
  }

  if (created)
    return (
      <div className="space-y-3">
        <Done>«{created}» creado.</Done>
        {/* Texto seguido, con las teclas dentro de la línea: con flex, cada trozo era
            una caja y quedaba un hueco antes de las comas. */}
        <p className="text-sm leading-7">
          Pruébalo: pulsa <Shortcut keys={hotkeyText(hotkey)} />, escribe «
          {created}» y pulsa <Keycap k="enter" />.
        </p>
        <p className="text-xs text-muted">
          Puedes añadirle apps, comandos y dónde colocar cada ventana en Ajustes
          → Espacios.
        </p>
      </div>
    );

  return (
    <div className="space-y-4">
      <p className="text-sm">
        Un Espacio abre de golpe todo lo de una tarea: carpetas, webs, apps y
        comandos. Crea uno sencillo; lo completas luego en Ajustes. O sigue sin
        él.
      </p>
      <Field label="Nombre">
        <TextInput
          placeholder="Trabajo"
          value={name}
          maxLength={80}
          onChange={(e) => setName(e.currentTarget.value)}
        />
      </Field>
      {roots.length > 0 && (
        <fieldset className="space-y-1">
          <legend className="pb-1 text-xs text-muted">Carpetas que abre</legend>
          {roots.map((r) => (
            <label key={r.id} className="flex items-center gap-2 text-sm">
              <input
                type="checkbox"
                checked={folders.includes(r.path)}
                onChange={(e) =>
                  setPicked(
                    e.currentTarget.checked
                      ? [...folders, r.path]
                      : folders.filter((p) => p !== r.path),
                  )
                }
                className="size-4 accent-accent"
              />
              <span className="truncate" title={r.path}>
                {folderName(r.path)}
              </span>
            </label>
          ))}
        </fieldset>
      )}
      <Field label="Una web que abre (opcional)">
        <TextInput
          placeholder="https://…"
          value={url}
          onChange={(e) => setUrl(e.currentTarget.value)}
        />
      </Field>
      <Button disabled={!draft} onClick={() => void create()}>
        Crear el Espacio
      </Button>
      {error && <ErrorText>{error}</ErrorText>}
    </div>
  );
}
