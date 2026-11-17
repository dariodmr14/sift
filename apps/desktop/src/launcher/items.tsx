// Las filas de la barra, vengan de donde vengan: Espacios, archivos, lo reciente y los
// comandos de la propia app. El orden lo deciden `mixResults` y las reglas de abajo.

import type { ReactNode } from "react";
import type { FileHit, RecentFile } from "../bindings";
import { Glyph, type GlyphName } from "../components/Glyph";
import { SpaceIcon } from "../components/SpaceIcon";
import { matchBuiltins, type BuiltinId } from "./builtins";
import { sectionStarts, shortFolder, type Section } from "./files";
import type { TypeFilter } from "./filters";
import { mixResults } from "./mix";
import type { SpaceView } from "./mode";
import { Highlighted } from "./panels";

/** Lo que la lista sabe pintar y ejecutar, venga de donde venga. */
export interface Item {
  key: string;
  section: Section;
  title: ReactNode;
  subtitle: ReactNode;
  icon: ReactNode;
  shortcut?: string;
  /** Ruta del archivo (solo archivos): Ctrl+↵ carpeta, Ctrl+Shift+C copiar. */
  path?: string;
  /** Qué hace al elegirla (datos, no una función: lo ejecuta la barra). */
  action: ItemAction;
}

export type ItemAction =
  | { kind: "space"; view: SpaceView }
  | { kind: "file"; path: string; name: string; page: number | null }
  | { kind: "settings"; newSpace: boolean };

const BUILTIN_ICON: Record<BuiltinId, GlyphName> = {
  settings: "settings",
  newSpace: "add",
};

function subtitleFor(view: SpaceView): string {
  const n = view.space.actions.length;
  const actions = n === 1 ? "1 acción" : `${n} acciones`;
  return view.space.aliases.length
    ? `${actions} · ${view.space.aliases.join(", ")}`
    : actions;
}

function FileIcon({ src }: { src: string | null | undefined }) {
  return src ? (
    <img src={src} alt="" className="size-6" draggable={false} />
  ) : (
    <span className="size-6" />
  );
}

export interface ItemSources {
  query: string;
  filter: TypeFilter | null;
  spaces: SpaceView[];
  fileHits: FileHit[];
  recent: RecentFile[];
  icons: Record<string, string | null>;
}

export function buildItems(s: ItemSources): Item[] {
  const { query, filter } = s;
  const mixed = mixResults(
    query,
    // Con un filtro de tipo solo salen archivos de ese tipo.
    (filter ? [] : s.spaces).map((v) => ({
      name: v.space.name,
      aliases: v.space.aliases,
      boost: v.boost ?? 1,
      view: v,
    })),
    s.fileHits,
  );
  const spaceItem = (view: SpaceView): Item => ({
    key: `space-${view.space.id}`,
    section: "spaces",
    title: view.space.name,
    subtitle: subtitleFor(view),
    icon: (
      <SpaceIcon
        icon={view.space.icon}
        autoIcon={view.autoIcon}
        name={view.space.name}
      />
    ),
    action: { kind: "space", view },
  });
  const fileItem = (hit: FileHit): Item => ({
    key: `file-${hit.fileId}`,
    section: "files",
    title: <Highlighted segments={hit.nameSegments} />,
    // Con coincidencia en el contenido, el fragmento; si no, la carpeta. Lo que llega
    // solo por significado no tiene palabras que resaltar: se dice, para que se
    // entienda por qué está ahí.
    subtitle: (
      <>
        {hit.byMeaning && (
          <span
            className="text-accent-ink"
            title="No contiene tus palabras: Sift lo ha encontrado por su significado"
          >
            Por significado ·{" "}
          </span>
        )}
        {hit.page != null && (
          <span title="Se abre en esta página">pág. {hit.page} · </span>
        )}
        {hit.snippet.length ? (
          <Highlighted segments={hit.snippet} />
        ) : (
          shortFolder(hit.path)
        )}
      </>
    ),
    icon: <FileIcon src={s.icons[hit.path]} />,
    path: hit.path,
    action: {
      kind: "file",
      path: hit.path,
      name: hit.name,
      page: hit.page ?? null,
    },
  });
  const toItem = (m: (typeof mixed)[number]) =>
    m.kind === "space" ? spaceItem(m.item.view) : fileItem(m.item);

  // Barra vacía: los Espacios y lo último que abriste, para llegar sin escribir. Con un
  // filtro y nada escrito aún, nada (la caja ya dice qué escribir).
  if (!query.trim() && filter) return [];
  if (!query.trim()) {
    const opened = s.recent.map((r): Item => ({
      key: `recent-${r.fileId}`,
      section: "recent",
      title: r.name,
      subtitle: shortFolder(r.path),
      icon: <FileIcon src={s.icons[r.path]} />,
      path: r.path,
      action: { kind: "file", path: r.path, name: r.name, page: null },
    }));
    return [...mixed.map(toItem), ...opened];
  }
  const builtins = (filter ? [] : matchBuiltins(query)).map((c): Item => ({
    key: `builtin-${c.id}`,
    section: "commands",
    title: c.title,
    subtitle: c.subtitle,
    icon: <Glyph name={BUILTIN_ICON[c.id]} className="text-xl" />,
    shortcut: c.shortcut,
    action: { kind: "settings", newSpace: c.id === "newSpace" },
  }));
  // Primero los Espacios que encajan exactamente, luego los comandos de la propia app y,
  // al final, archivos con los Espacios que encajan a medias intercalados (ver
  // `mixResults`). Los comandos solo salen si lo escrito encaja con su nombre
  // («ajustes»), y son uno o dos: detrás de cientos de archivos no se encontrarían.
  const exact = mixed.filter((m) => m.kind === "space" && m.exact);
  const rest = mixed.filter((m) => !(m.kind === "space" && m.exact));
  const list = [...exact.map(toItem), ...builtins, ...rest.map(toItem)];
  // Sin cabeceras (Espacios intercalados entre archivos), cada Espacio dice qué es.
  if (sectionStarts(list.map((i) => i.section)).size === 0) {
    return list.map((i) =>
      i.section === "spaces"
        ? { ...i, subtitle: <>Espacio · {i.subtitle}</> }
        : i,
    );
  }
  return list;
}
