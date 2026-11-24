import { useCallback, useEffect, useState } from "react";
import {
  commands,
  events,
  type IndexStatus,
  type SuggestedKind,
  type WelcomeView,
} from "../bindings";

/** Tamaño del modelo si Rust aún no lo ha dicho (lo comprueba un test de `sift-embeddings`: `card.rs`). */
const FALLBACK_MB = 135;

/** Lo que enseña la bienvenida: atajo, carpetas sugeridas y estado del modelo. */
export function useWelcome() {
  const [view, setView] = useState<WelcomeView | null>(null);
  const [status, setStatus] = useState<IndexStatus | null>(null);
  const [error, setError] = useState<string | null>(null);
  // «Ahora no» y «Descargar»: respuestas de esta sesión (la descarga la confirma Rust).
  const [skipped, setSkipped] = useState(false);
  const [requested, setRequested] = useState(false);

  const reload = useCallback(
    () =>
      commands.getWelcome().then((r) => {
        if (r.status === "ok") setView(r.data);
        else setError(r.error.message);
      }),
    [],
  );

  useEffect(() => {
    void reload();
    void commands.indexStatus().then(setStatus);
    const unlisten = events.indexProgress.listen((e) => setStatus(e.payload));
    return () => {
      void unlisten.then((stop) => stop());
    };
  }, [reload]);

  async function addSuggested(kind: SuggestedKind) {
    const r = await commands.addSuggestedFolder(kind);
    setError(r.status === "error" ? r.error.message : null);
    await reload();
  }

  async function addOther() {
    const r = await commands.addRoot();
    setError(r.status === "error" ? r.error.message : null);
    await reload();
  }

  async function allowDownload() {
    setRequested(true);
    const r = await commands.allowModelDownload();
    if (r.status === "error") {
      setRequested(false);
      setError(r.error.message);
    }
  }

  const smart = status?.smart ?? null;
  return {
    view,
    error,
    status,
    smart,
    downloadMb: smart?.downloadMb || FALLBACK_MB,
    skipped,
    requested,
    addSuggested,
    addOther,
    allowDownload,
    skip: () => setSkipped(true),
  };
}
