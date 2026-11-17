import { useEffect, useMemo, useState } from "react";
import { commands, type InstalledApp } from "../bindings";
import { normalize } from "../launcher/text";
import { ErrorText, Modal, TextInput } from "./ui";

/** Cuántas apps se pintan a la vez; con la búsqueda se llega al resto. */
const VISIBLE = 60;

/**
 * Selector de aplicaciones instaladas (las del menú Inicio, incluidas las de la Store).
 * Sin rutas a mano: eliges por nombre y Sift guarda su identificador de Windows.
 */
export function AppPicker({
  onPick,
  onClose,
}: {
  onPick: (app: InstalledApp) => void;
  onClose: () => void;
}) {
  const [apps, setApps] = useState<InstalledApp[] | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [query, setQuery] = useState("");
  const [icons, setIcons] = useState<Record<string, string | null>>({});
  const [selected, setSelected] = useState(0);

  useEffect(() => {
    void commands.listApps().then((r) => {
      if (r.status === "ok") setApps(r.data);
      else setError(r.error.message);
    });
  }, []);

  const visible = useMemo(() => {
    const q = normalize(query.trim());
    const list = apps ?? [];
    return (q ? list.filter((a) => normalize(a.name).includes(q)) : list).slice(
      0,
      VISIBLE,
    );
  }, [apps, query]);

  // Iconos solo de lo que se ve, y una vez por app.
  useEffect(() => {
    const missing = visible.filter((a) => !(a.id in icons));
    if (missing.length === 0) return;
    let alive = true;
    void commands.getIcons(missing.map((a) => `app:${a.id}`)).then((r) => {
      if (!alive || r.status !== "ok") return;
      setIcons((prev) => {
        const next = { ...prev };
        missing.forEach((a, i) => (next[a.id] = r.data[i] ?? null));
        return next;
      });
    });
    return () => {
      alive = false;
    };
  }, [visible, icons]);

  function onKeyDown(e: React.KeyboardEvent) {
    if (e.key === "ArrowDown" || e.key === "ArrowUp") {
      e.preventDefault();
      const delta = e.key === "ArrowDown" ? 1 : -1;
      setSelected((i) =>
        Math.min(Math.max(i + delta, 0), Math.max(visible.length - 1, 0)),
      );
    } else if (e.key === "Enter" && visible[selected]) {
      e.preventDefault();
      onPick(visible[selected]);
    }
  }

  return (
    <Modal title="Elegir aplicación" onClose={onClose}>
      <div className="space-y-2 p-4" onKeyDown={onKeyDown}>
        <TextInput
          autoFocus
          placeholder="Buscar aplicación"
          value={query}
          onChange={(e) => {
            setQuery(e.currentTarget.value);
            setSelected(0);
          }}
        />
        {error && <ErrorText>{error}</ErrorText>}
      </div>
      <ul
        role="listbox"
        aria-label="Aplicaciones"
        className="min-h-40 overflow-y-auto px-2 pb-2"
      >
        {apps === null && !error && (
          <li className="p-3 text-sm text-muted">Cargando aplicaciones…</li>
        )}
        {apps !== null && visible.length === 0 && (
          <li className="p-3 text-sm text-muted">
            Ninguna aplicación con ese nombre
          </li>
        )}
        {visible.map((app, i) => (
          <li
            key={app.id}
            role="option"
            aria-selected={i === selected}
            onMouseMove={() => setSelected(i)}
            onClick={() => onPick(app)}
            className="sift-option flex cursor-default items-center gap-3 px-3 py-1.5 text-sm"
          >
            {icons[app.id] ? (
              <img src={icons[app.id]!} alt="" className="size-6" />
            ) : (
              <span className="size-6" aria-hidden />
            )}
            {app.name}
          </li>
        ))}
      </ul>
    </Modal>
  );
}
