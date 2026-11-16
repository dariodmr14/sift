// Lo que la barra necesita de Rust y se recarga cada vez que se abre: los Espacios, lo
// abierto hace poco y el ajuste de iconos. Así siempre refleja lo último de Ajustes.

import { useCallback, useEffect, useState } from "react";
import { commands, type RecentFile } from "../bindings";
import type { SpaceView } from "./mode";

export function useBarData() {
  const [spaces, setSpaces] = useState<SpaceView[]>([]);
  // «Abierto recientemente». No se borra al ocultarse: al abrir se pinta con la lista de
  // la vez anterior y se pone al día después, sin esperar a Rust.
  const [recent, setRecent] = useState<RecentFile[]>([]);
  const [iconsHidden, setIconsHidden] = useState(false);

  const load = useCallback(
    (alive: () => boolean = () => true) =>
      Promise.all([
        commands.listSpaces(),
        commands.getSettings(),
        commands.recentFiles(),
      ]).then(([list, settings, opened]) => {
        if (!alive()) return; // el componente ya no existe: no tocar su estado
        if (list.status === "ok") setSpaces(list.data);
        setIconsHidden(settings.iconsHidden);
        if (opened.status === "ok") setRecent(opened.data);
      }),
    [],
  );

  useEffect(() => {
    let alive = true;
    void load(() => alive);
    return () => {
      alive = false;
    };
  }, [load]);

  return { spaces, recent, iconsHidden, refresh: () => load() };
}
