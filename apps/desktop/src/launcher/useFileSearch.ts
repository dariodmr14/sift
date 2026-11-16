// Búsqueda de archivos en Rust a cada tecla (es rápida: < 50 ms) y sus iconos de Windows.

import { useEffect, useRef, useState } from "react";
import { commands, type FileHit } from "../bindings";
import type { TypeFilter } from "./filters";
import { isLatest } from "./files";

export function useFileSearch(query: string, filter: TypeFilter | null) {
  const [fileHits, setFileHits] = useState<FileHit[]>([]);
  // Número de la última búsqueda pedida: las respuestas de búsquedas anteriores se ignoran.
  const searchSeq = useRef(0);

  useEffect(() => {
    const seq = ++searchSeq.current;
    if (!query.trim()) {
      // Sin consulta no hay archivos que mostrar (se limpia en la siguiente vuelta).
      void Promise.resolve().then(
        () => isLatest(seq, searchSeq.current) && setFileHits([]),
      );
      return;
    }
    void commands.searchFiles(query, seq, filter?.id ?? null).then((r) => {
      if (r.status !== "ok" || !isLatest(r.data.seq, searchSeq.current)) return;
      setFileHits(r.data.results.hits);
    });
  }, [query, filter]);

  return fileHits;
}

/** Iconos de Windows de unas rutas (una vez por ruta; se guardan entre búsquedas). */
export function useFileIcons(paths: string[]) {
  const [icons, setIcons] = useState<Record<string, string | null>>({});
  const key = paths.join("\n");

  useEffect(() => {
    const missing = key
      .split("\n")
      .filter((p, i, all) => p && !(p in icons) && all.indexOf(p) === i);
    if (missing.length === 0) return;
    let alive = true;
    void commands.getIcons(missing).then((r) => {
      if (!alive || r.status !== "ok") return;
      setIcons((prev) => ({
        ...prev,
        ...Object.fromEntries(missing.map((p, i) => [p, r.data[i] ?? null])),
      }));
    });
    return () => {
      alive = false;
    };
  }, [key, icons]);

  return icons;
}
