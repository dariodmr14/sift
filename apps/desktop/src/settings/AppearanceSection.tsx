import { useEffect, useState } from "react";
import {
  commands,
  events,
  type GlassBlocker,
  type ThemeChoice,
} from "../bindings";
import { Field, Select } from "./ui";

/** Ajustes → General → Apariencia: tema, iconos de la barra y vidrio. */
export function AppearanceSection({
  theme,
  iconsHidden,
  disableGlass,
}: {
  theme: ThemeChoice;
  iconsHidden: boolean;
  disableGlass: boolean;
}) {
  return (
    <section className="space-y-3">
      <h2 className="text-sm font-semibold">Apariencia</h2>
      <ThemeSelect initial={theme} />
      <IconsToggle initial={iconsHidden} />
      <GlassToggle initial={disableGlass} />
    </section>
  );
}

const THEMES: { value: ThemeChoice; label: string }[] = [
  { value: "system", label: "Como Windows" },
  { value: "light", label: "Claro" },
  { value: "dark", label: "Oscuro" },
];

/** «Tema». Se aplica al momento en toda Sift: Rust avisa a cada ventana (`appearanceChanged`). */
function ThemeSelect({ initial }: { initial: ThemeChoice }) {
  const [theme, setTheme] = useState(initial);
  return (
    <Field label="Tema">
      <Select
        value={theme}
        onChange={(e) => {
          const next = e.currentTarget.value as ThemeChoice;
          setTheme(next);
          void commands.setTheme(next).then((r) => {
            if (r.status === "ok") setTheme(r.data);
            else setTheme(theme);
          });
        }}
        className="w-48"
      >
        {THEMES.map((t) => (
          <option key={t.value} value={t.value}>
            {t.label}
          </option>
        ))}
      </Select>
    </Field>
  );
}

function IconsToggle({ initial }: { initial: boolean }) {
  const [hidden, setHidden] = useState(initial);
  return (
    <label className="flex items-center gap-3">
      <input
        type="checkbox"
        checked={hidden}
        onChange={(e) => {
          const next = e.currentTarget.checked;
          void commands
            .setIconsHidden(next)
            .then((r) => r.status === "ok" && setHidden(r.data));
        }}
        className="size-4 accent-accent"
      />
      Solo texto: ocultar todos los iconos en la barra
    </label>
  );
}

/** Por qué no se ve el vidrio, en palabras de cualquiera. */
export const GLASS_BLOCKED: Record<GlassBlocker, string> = {
  transparencyOff:
    "los efectos de transparencia de Windows están desactivados (Configuración → Personalización → Colores).",
  sandbox: "Sift se está ejecutando en Windows Sandbox.",
  remoteSession: "estás conectado a este PC por escritorio remoto.",
  noGpu:
    "Windows está dibujando la pantalla sin la tarjeta gráfica (pasa en máquinas virtuales o si falta su controlador).",
  batterySaver: "el ahorro de batería está activado.",
};

/** Si ahora mismo Windows no puede pintar el vidrio, el motivo (o `null`). */
function useGlassBlocker(): GlassBlocker | null {
  const [blocker, setBlocker] = useState<GlassBlocker | null>(null);
  useEffect(() => {
    const load = () => void commands.getGlassBlocker().then(setBlocker);
    load();
    const changed = events.appearanceChanged.listen(load);
    window.addEventListener("focus", load);
    return () => {
      void changed.then((stop) => stop());
      window.removeEventListener("focus", load);
    };
  }, []);
  return blocker;
}

function GlassToggle({ initial }: { initial: boolean }) {
  const [disabled, setDisabled] = useState(initial);
  const blocker = useGlassBlocker();
  return (
    <label className="flex items-start gap-3">
      <input
        type="checkbox"
        checked={disabled}
        onChange={(e) => {
          const next = e.currentTarget.checked;
          void commands.setGlassDisabled(next).then((r) => {
            if (r.status === "ok") setDisabled(r.data);
          });
        }}
        className="mt-1 size-4 accent-accent"
      />
      <span>
        Desactivar vidrio
        <span className="block text-xs text-muted">
          La barra, los ajustes y los diálogos pasan a ser opacos: no se ve lo
          que hay detrás.
        </span>
        {!disabled && blocker && (
          <span role="status" className="mt-1 block text-xs text-muted">
            Ahora mismo el vidrio no se usa: {GLASS_BLOCKED[blocker]} Sift pone
            un fondo opaco para que todo se lea bien, y el vidrio vuelve solo
            cuando Windows pueda pintarlo.
          </span>
        )}
      </span>
    </label>
  );
}
