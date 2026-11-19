import { useEffect, useMemo, useState } from "react";
import { commands } from "../bindings";
import { SpaceIcon, svgDataUrl } from "../components/SpaceIcon";
import type { Icon } from "./draft";
import { buildSvg, searchIcons, type IconNode } from "./iconLibrary";
import { Button, ErrorText, TextInput } from "./ui";
import { SiftLoading } from "../components/SiftLoader";

type Tab = Icon["kind"];

const TABS: { id: Tab; label: string }[] = [
  { id: "auto", label: "Automático" },
  { id: "library", label: "Librería" },
  { id: "emoji", label: "Emoji" },
  { id: "image", label: "Imagen" },
  { id: "none", label: "Ninguno" },
];

const EMOJI_IDEAS = [
  "🗄️",
  "📚",
  "💻",
  "🎓",
  "🧪",
  "🎮",
  "🎵",
  "🌐",
  "🚀",
  "📝",
  "🛠️",
  "🎨",
  "📊",
  "🐳",
  "☕",
  "⭐",
];

/** Lucide se carga la primera vez que se abre la pestaña "Librería" (no antes). */
let libraryPromise: Promise<Record<string, IconNode>> | null = null;
function loadLibrary() {
  libraryPromise ??= import("lucide").then(
    (m) => m.icons as unknown as Record<string, IconNode>,
  );
  return libraryPromise;
}

interface Props {
  icon: Icon;
  autoIcon?: string | null;
  name: string;
  onChange: (icon: Icon) => void;
}

export function IconPicker({ icon, autoIcon, name, onChange }: Props) {
  const [tab, setTab] = useState<Tab>(icon.kind);

  return (
    <div className="space-y-3">
      <div className="flex items-center gap-3">
        <span className="flex size-10 items-center justify-center rounded-item bg-hover">
          <SpaceIcon
            icon={icon}
            autoIcon={autoIcon}
            name={name || "?"}
            size={28}
          />
        </span>
        <div
          role="tablist"
          aria-label="Tipo de icono"
          className="flex flex-wrap gap-1"
        >
          {TABS.map((t) => (
            <Button
              key={t.id}
              role="tab"
              aria-selected={tab === t.id}
              variant={tab === t.id ? "primary" : "ghost"}
              onClick={() => {
                setTab(t.id);
                if (t.id === "auto" || t.id === "none")
                  onChange({ kind: t.id });
              }}
            >
              {t.label}
            </Button>
          ))}
        </div>
      </div>

      {tab === "auto" && (
        <p className="text-xs text-muted">
          El icono real de Windows de la primera app, archivo o carpeta del
          Espacio.
        </p>
      )}
      {tab === "library" && (
        <LibraryTab
          selected={icon.kind === "library" ? icon.name : null}
          onChange={onChange}
        />
      )}
      {tab === "emoji" && (
        <EmojiTab
          value={icon.kind === "emoji" ? icon.value : ""}
          onChange={onChange}
        />
      )}
      {tab === "image" && (
        <ImageTab onChange={onChange} hasImage={icon.kind === "image"} />
      )}
    </div>
  );
}

function LibraryTab({
  selected,
  onChange,
}: {
  selected: string | null;
  onChange: (i: Icon) => void;
}) {
  const [library, setLibrary] = useState<Record<string, IconNode> | null>(null);
  const [query, setQuery] = useState("");

  useEffect(() => {
    let alive = true;
    void loadLibrary().then((lib) => alive && setLibrary(lib));
    return () => {
      alive = false;
    };
  }, []);

  const hits = useMemo(
    () => (library ? searchIcons(query, Object.keys(library), 96) : []),
    [library, query],
  );

  return (
    <div className="space-y-2">
      <TextInput
        placeholder="Buscar icono"
        title="En español o en inglés: carpeta, base, código…"
        value={query}
        onChange={(e) => setQuery(e.currentTarget.value)}
      />
      {!library ? (
        <SiftLoading label="Cargando iconos…" delay={300} />
      ) : (
        <div className="grid max-h-56 grid-cols-12 gap-1 overflow-y-auto">
          {hits.map((hit) => {
            const svg = buildSvg(library[hit.name]);
            return (
              <button
                key={hit.name}
                type="button"
                title={hit.label}
                aria-label={hit.label}
                aria-pressed={selected === hit.name}
                onClick={() =>
                  onChange({ kind: "library", name: hit.name, svg })
                }
                className={`flex aspect-square items-center justify-center rounded-item hover:bg-selected ${
                  selected === hit.name
                    ? "bg-selected ring-2 ring-accent-ink"
                    : ""
                }`}
              >
                <span
                  aria-hidden
                  className="size-5"
                  style={{
                    backgroundColor: "currentColor",
                    mask: `url("${svgDataUrl(svg)}") center / contain no-repeat`,
                  }}
                />
              </button>
            );
          })}
          {hits.length === 0 && (
            <p className="col-span-12 text-xs text-muted">
              Ningún icono con ese nombre
            </p>
          )}
        </div>
      )}
    </div>
  );
}

function EmojiTab({
  value,
  onChange,
}: {
  value: string;
  onChange: (i: Icon) => void;
}) {
  return (
    <div className="space-y-2">
      <div className="flex flex-wrap gap-1">
        {EMOJI_IDEAS.map((e) => (
          <button
            key={e}
            type="button"
            onClick={() => onChange({ kind: "emoji", value: e })}
            className={`size-9 rounded-item text-xl hover:bg-selected ${value === e ? "ring-2 ring-accent-ink" : ""}`}
          >
            {e}
          </button>
        ))}
      </div>
      <TextInput
        className="max-w-48"
        placeholder="O escribe uno (Win + .)"
        value={value}
        maxLength={8}
        onChange={(e) => {
          const v = e.currentTarget.value;
          onChange(
            v.trim() ? { kind: "emoji", value: v.trim() } : { kind: "auto" },
          );
        }}
      />
    </div>
  );
}

function ImageTab({
  hasImage,
  onChange,
}: {
  hasImage: boolean;
  onChange: (i: Icon) => void;
}) {
  const [error, setError] = useState<string | null>(null);
  async function pick() {
    const r = await commands.pickIconImage();
    if (r.status === "error") setError(r.error.message);
    else if (r.data) {
      setError(null);
      onChange(r.data);
    }
  }
  return (
    <div className="space-y-2">
      <Button onClick={() => void pick()}>
        {hasImage ? "Cambiar imagen…" : "Elegir imagen…"}
      </Button>
      <p className="text-xs text-muted">
        PNG, SVG o ICO, hasta ~256 KB. Se guarda dentro del Espacio.
      </p>
      {error && <ErrorText>{error}</ErrorText>}
    </div>
  );
}
