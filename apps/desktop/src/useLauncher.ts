import { useEffect, useRef } from "react";
import { commands, events } from "./bindings";

/**
 * Conecta la UI del lanzador con Rust:
 * - Pide el estado una vez para saber si hay Acrylic (si no, el CSS deja el fondo opaco).
 * - Cada vez que Rust muestra la ventana (`launcherShown`), llama a `onShown` (limpiar la
 *   búsqueda y enfocar la caja) y, tras pintar, avisa a Rust para medir la latencia.
 */
export function useLauncher(onShown: () => void) {
  // Guardamos el callback en una ref para no volver a suscribirnos en cada render.
  const onShownRef = useRef(onShown);
  useEffect(() => {
    onShownRef.current = onShown;
  });

  useEffect(() => {
    void commands.getStatus().then((status) => {
      document.documentElement.dataset.acrylic = String(status.acrylic);
    });

    const unlisten = events.launcherShown.listen(() => {
      onShownRef.current();
      // Dos requestAnimationFrame: el primero se ejecuta antes de pintar el siguiente
      // fotograma; el segundo, cuando ese fotograma ya se ha pintado.
      requestAnimationFrame(() =>
        requestAnimationFrame(() => void commands.launcherPainted()),
      );
    });
    return () => {
      void unlisten.then((stop) => stop());
    };
  }, []);
}
