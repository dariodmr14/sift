// La vista previa de un resultado: se pide a Rust solo con la vista previa abierta, y lo
// ya pedido se guarda para que volver a un resultado sea instantáneo.

import { useEffect, useRef, useState } from "react";
import { commands, type FileHit, type FilePreview } from "../bindings";

export type PreviewState =
  | { status: "loading" }
  | { status: "ready"; preview: FilePreview }
  | { status: "error"; message: string };

export function usePreview(
  hit: FileHit | undefined,
  query: string,
  open: boolean,
): PreviewState | null {
  const cache = useRef(new Map<string, FilePreview>());
  // La respuesta guardada con la clave a la que pertenece: si no coincide con la
  // pedida ahora, es una vieja y se ignora.
  const [answer, setAnswer] = useState<{
    key: string;
    state: PreviewState;
  } | null>(null);
  const fileId = hit?.fileId;
  const chunkId = hit?.chunkId;
  const key =
    open && fileId !== undefined ? `${fileId}:${chunkId}:${query}` : null;

  useEffect(() => {
    if (key === null || fileId === undefined || chunkId === undefined) return;
    const cached = cache.current.get(key);
    if (cached) {
      void Promise.resolve().then(() =>
        setAnswer({ key, state: { status: "ready", preview: cached } }),
      );
      return;
    }
    let alive = true;
    void commands.previewFile(fileId, chunkId, query).then((r) => {
      if (!alive) return;
      if (r.status === "ok") {
        cache.current.set(key, r.data);
        setAnswer({ key, state: { status: "ready", preview: r.data } });
      } else {
        setAnswer({
          key,
          state: { status: "error", message: r.error.message },
        });
      }
    });
    return () => {
      alive = false;
    };
  }, [key, fileId, chunkId, query]);

  if (key === null) return null;
  return answer?.key === key ? answer.state : { status: "loading" };
}
