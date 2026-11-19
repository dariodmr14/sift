// Una ilustración corta por paso de la introducción (FEEDBACK 2026-11-19, 6c): se ve una
// vez al entrar en el paso (menos de 1,6 s) y se queda en su último fotograma. Solo
// opacidad y transformaciones; con «reducir movimiento», directamente el último fotograma.
// Es decorativa (`aria-hidden`): lo que cuenta ya lo dicen el título y el texto.

import { useEffect, useState, type CSSProperties } from "react";
import { Glyph } from "../components/Glyph";
import type { IntroStepId } from "./introSteps";

/** Retraso de cada pieza, en ms (`--d` en `.sift-art`). */
const at = (ms: number) => ({ "--d": `${ms}ms` }) as CSSProperties;

function reducedMotion(): boolean {
  return (
    typeof window !== "undefined" &&
    typeof window.matchMedia === "function" &&
    window.matchMedia("(prefers-reduced-motion: reduce)").matches
  );
}

/**
 * Recorre `frames` uno a uno: el segundo a los `delay` ms y los demás cada `every` ms.
 * Con «reducir movimiento», el último desde el principio.
 */
export function useTicker<T>(
  frames: T[],
  delay: number,
  every: number,
): T | undefined {
  const [i, setI] = useState(() => (reducedMotion() ? frames.length - 1 : 0));
  useEffect(() => {
    if (reducedMotion()) return;
    const timers = frames
      .slice(1)
      .map((_, k) => setTimeout(() => setI(k + 1), delay + k * every));
    return () => timers.forEach(clearTimeout);
    // Los fotogramas son fijos para cada paso: solo se reproduce al montar.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);
  return frames[i];
}

/** Paso 1: las teclas del atajo se hunden por turnos y baja la barra. */
function HotkeyArt({ keys }: { keys: string[] }) {
  return (
    <>
      <div
        className="sift-art-drop absolute inset-x-6 top-3 flex h-7 items-center gap-2 rounded-item border border-line bg-hover px-3 text-xs text-muted"
        style={at(750)}
      >
        <Glyph name="search" className="text-xs" />
        Buscar
      </div>
      <div className="absolute inset-x-0 bottom-5 flex justify-center gap-1">
        {keys.map((k, i) => (
          <kbd
            key={i}
            className="sift-key sift-art-press"
            style={at(100 + i * 150)}
          >
            {k}
          </kbd>
        ))}
      </div>
    </>
  );
}

const COUNT = [0, 96, 241, 418, 655, 902, 1103, 1284].map(
  (n) => `${n.toLocaleString("es-ES")} archivos`,
);

/** Paso 2: tres carpetas llegan volando y sube el contador. */
function FoldersArt() {
  const count = useTicker(COUNT, 300, 120);
  return (
    <>
      <div className="absolute inset-y-0 left-8 flex items-center gap-6 text-xl text-accent-ink">
        {[0, 1, 2].map((i) => (
          <span key={i} className="sift-art-fly" style={at(100 + i * 200)}>
            <Glyph name="folder" className="text-2xl" />
          </span>
        ))}
      </div>
      <span className="absolute right-5 bottom-4 text-sm tabular-nums">
        {count}
      </span>
    </>
  );
}

const WORD = "factura";
const TYPED = Array.from({ length: WORD.length + 1 }, (_, i) =>
  WORD.slice(0, i),
);

/** Paso 3: se escribe «factura» y caen tres resultados. */
function SearchArt() {
  const typed = useTicker(TYPED, 150, 90);
  return (
    <>
      <div className="absolute inset-x-6 top-3 flex h-7 items-center gap-2 rounded-item border border-line bg-hover px-3 text-xs">
        <Glyph name="search" className="text-xs text-muted" />
        {typed}
      </div>
      {[0, 1, 2].map((i) => (
        <div
          key={i}
          className="sift-art-fall absolute left-6 h-3 rounded-[2px] bg-selected"
          style={{ ...at(900 + i * 150), top: 52 + i * 20, right: 24 + i * 40 }}
        />
      ))}
    </>
  );
}

/** Paso 4: una línea une «viaje» con «billetes de tren.pdf». */
function SmartArt() {
  return (
    <div className="absolute inset-0 flex items-center justify-center gap-2 text-sm">
      <span>«viaje»</span>
      <span
        className="sift-art-grow h-0.5 w-12 origin-left bg-accent"
        style={at(500)}
      />
      <span
        className="sift-art-fade flex items-center gap-1 rounded-item bg-selected px-2 py-1 text-xs"
        style={at(1000)}
      >
        <Glyph name="document" className="text-xs" />
        billetes de tren.pdf
      </span>
    </div>
  );
}

/** Paso 5: la tarjeta de un Espacio se abre en tres ventanas colocadas. */
function SpaceArt() {
  return (
    <>
      <div className="absolute inset-0 flex items-center justify-center">
        <div
          className="sift-art-open flex items-center gap-2 rounded-item border border-line bg-hover px-3 py-1.5 text-xs"
          style={at(200)}
        >
          <Glyph name="apps" className="text-xs" />
          Trabajo
        </div>
      </div>
      <div className="absolute inset-2 grid grid-cols-2 grid-rows-2 gap-2">
        <span
          className="sift-art-window-l row-span-2 rounded-[4px] border border-accent bg-selected"
          style={at(550)}
        />
        <span
          className="sift-art-window-r rounded-[4px] border border-accent bg-selected"
          style={at(700)}
        />
        <span
          className="sift-art-window-r rounded-[4px] border border-accent bg-selected"
          style={at(850)}
        />
      </div>
    </>
  );
}

export function IntroArt({ id, keys }: { id: IntroStepId; keys: string[] }) {
  return (
    <div
      aria-hidden
      data-art={id}
      className="sift-art relative h-28 overflow-hidden rounded-item border border-line bg-hover/40"
    >
      {id === "hotkey" && <HotkeyArt keys={keys} />}
      {id === "folders" && <FoldersArt />}
      {id === "search" && <SearchArt />}
      {id === "smart" && <SmartArt />}
      {id === "space" && <SpaceArt />}
    </div>
  );
}
