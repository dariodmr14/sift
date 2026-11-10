import { useEffect, useRef, type RefObject } from "react";
import { commands, events } from "./bindings";
import {
  applyAppearance,
  applyWindowMaterial,
  prefersReducedMotion,
} from "./appearance";

/** Reinicia una animación CSS: quita las clases, fuerza un reflow y pone la nueva. */
function playMotion(
  el: HTMLElement | null,
  cls: "sift-enter" | "sift-exit" | "sift-from",
) {
  if (!el) return;
  el.classList.remove("sift-enter", "sift-exit", "sift-from");
  void el.offsetWidth; // sin esto, el navegador no vuelve a empezar la animación
  el.classList.add(cls);
}

/**
 * Conecta la UI del lanzador con Rust:
 * - Al arrancar: Acrylic sí/no, tema y acento de Windows.
 * - Al mostrarse (`launcherShown`): `onShown` (limpiar y enfocar), animación de entrada,
 *   tema/acento al día (por si cambiaron) y, tras pintar, aviso para medir la latencia.
 * - Al ocultarse (`launcherHidden`): `onHidden` y vuelta al punto de partida de la
 *   entrada, mientras la ventana es invisible: así no se ve un fotograma viejo al abrir.
 *
 */
export function useLauncher(
  root: RefObject<HTMLElement | null>,
  onShown: () => void,
  onHidden: () => void,
) {
  // Callbacks en refs para no volver a suscribirse en cada render.
  const shownRef = useRef(onShown);
  const hiddenRef = useRef(onHidden);
  useEffect(() => {
    shownRef.current = onShown;
    hiddenRef.current = onHidden;
  });

  useEffect(() => {
    void applyWindowMaterial();
    void applyAppearance();

    const shown = events.launcherShown.listen(() => {
      shownRef.current();
      playMotion(root.current, "sift-enter");
      void applyAppearance();
      // Dos requestAnimationFrame: el primero se ejecuta antes de pintar el siguiente
      // fotograma; el segundo, cuando ese fotograma ya se ha pintado.
      requestAnimationFrame(() =>
        requestAnimationFrame(() => void commands.launcherPainted()),
      );
    });
    const hidden = events.launcherHidden.listen(() => {
      hiddenRef.current();
      playMotion(root.current, "sift-from");
    });
    return () => {
      void shown.then((stop) => stop());
      void hidden.then((stop) => stop());
    };
  }, [root]);
}

/**
 * Cierra la barra con la animación de salida, o al instante con «reducir movimiento».
 * Función normal (no un hook): se le pasa el elemento raíz al usarla.
 */
export function dismissLauncher(el: HTMLElement | null) {
  if (!el || prefersReducedMotion()) {
    void commands.hideLauncher();
    return;
  }
  el.addEventListener("animationend", () => void commands.hideLauncher(), {
    once: true,
  });
  playMotion(el, "sift-exit");
}
