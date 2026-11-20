// Abrir un Espacio desde la barra: el panel «en marcha» (solo si tarda), su progreso,
// cancelar con Esc y qué pasa si la barra se oculta mientras tanto (ADR 0012).

import {
  useCallback,
  useEffect,
  useRef,
  type Dispatch,
  type SetStateAction,
} from "react";
import { commands, events } from "../bindings";
import {
  addStep,
  failedMode,
  setPlacement,
  type Mode,
  type RunStep,
  type SpaceView,
} from "./mode";
import type { SpaceProgress } from "./panels";
import { summarize } from "./runSummary";

/** Por debajo de esto no se enseña el panel «en marcha»: no debe parpadear. */
const SLOW_MS = 300;

export function useSpaceRun({
  setMode,
  onDone,
  onResetHidden,
}: {
  setMode: Dispatch<SetStateAction<Mode>>;
  /** Todo fue bien con la barra a la vista: se cierra. */
  onDone: () => void;
  /** Todo fue bien con la barra oculta: se deja limpia para la próxima vez. */
  onResetHidden: () => void;
}) {
  // Un Espacio con esperas puede seguir en marcha con la barra oculta. Mientras tanto no
  // se limpia nada, y si al acabar hay algo que contar, se enseña al volver a abrirla.
  const running = useRef(false);
  const visible = useRef(true);
  const keepResult = useRef(false);
  const cancelRequested = useRef(false);
  const callbacks = useRef({ onDone, onResetHidden });
  useEffect(() => {
    callbacks.current = { onDone, onResetHidden };
  });

  // Por qué paso va el Espacio en marcha. Se apunta desde el principio, aunque el panel
  // aún no se vea (sale a los 300 ms): si no, faltarían los primeros pasos.
  const seen = useRef<{ progress: SpaceProgress | null; steps: RunStep[] }>({
    progress: null,
    steps: [],
  });
  useEffect(() => {
    const show = () =>
      setMode((m) => (m.kind === "running" ? { ...m, ...seen.current } : m));
    const stopProgress = events.spaceProgress.listen(({ payload }) => {
      seen.current = {
        progress: payload,
        steps: addStep(seen.current.steps, payload),
      };
      show();
    });
    // Las ventanas se colocan en segundo plano: «abierto · colocando…» y luego ✓.
    const stopPlacement = events.spacePlacement.listen(({ payload }) => {
      seen.current = {
        ...seen.current,
        steps: setPlacement(seen.current.steps, payload),
      };
      show();
    });
    return () => {
      void stopProgress.then((f) => f());
      void stopPlacement.then((f) => f());
    };
  }, [setMode]);

  const launch = useCallback(
    async (view: SpaceView) => {
      // Un Enter doble (o doble clic) antes de que salga el panel no lo abre dos veces.
      if (running.current) return;
      const title = view.space.name;
      running.current = true;
      cancelRequested.current = false;
      seen.current = { progress: null, steps: [] };
      const slow = setTimeout(
        () =>
          setMode({
            kind: "running",
            title,
            ...seen.current,
            cancelling: false,
          }),
        SLOW_MS,
      );
      // Si la llamada falla del todo (no un error de Sift, sino de la comunicación), la
      // barra no puede quedarse atascada en «Abriendo…»: se dice y se libera (auditoría 1.0).
      let result: Awaited<ReturnType<typeof commands.runSpace>>;
      try {
        result = await commands.runSpace(view.space.id);
      } catch (e) {
        setMode(failedMode(title, String(e)));
        return;
      } finally {
        clearTimeout(slow);
        running.current = false;
      }
      if (result.status === "error") {
        setMode(failedMode(title, result.error.message));
        return;
      }
      const report = result.data;
      // Había comandos nuevos y en el diálogo de Windows se eligió «Cancelar»: no se
      // lanzó nada, la barra vuelve a la lista.
      if (report.status === "declined") {
        setMode({ kind: "list" });
        return;
      }
      const summary = summarize(report.outcomes, cancelRequested.current);
      if (summary.needsAttention) {
        setMode({ kind: "result", title, ...summary });
        // Con la barra oculta, que lo vea al volver a abrirla.
        if (!visible.current) keepResult.current = true;
      } else if (visible.current) {
        callbacks.current.onDone();
      } else {
        callbacks.current.onResetHidden();
      }
    },
    [setMode],
  );

  /** Esc mientras se abre: lo ya abierto sigue abierto; al acabar, el resumen. */
  const cancel = useCallback(() => {
    cancelRequested.current = true;
    void commands.cancelSpaceRun();
    setMode((m) => (m.kind === "running" ? { ...m, cancelling: true } : m));
  }, [setMode]);

  /** La barra se ha abierto: ¿hay que conservar lo que enseña (en marcha o resumen)? */
  const shown = useCallback((): boolean => {
    visible.current = true;
    const keep = running.current || keepResult.current;
    keepResult.current = false;
    return keep;
  }, []);

  /** La barra se ha ocultado: ¿se puede limpiar ya (no hay nada en marcha)? */
  const hidden = useCallback((): boolean => {
    visible.current = false;
    return !running.current;
  }, []);

  return { launch, cancel, shown, hidden };
}
