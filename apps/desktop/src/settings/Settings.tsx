import { useEffect, useState } from "react";
import {
  commands,
  events,
  type SettingsView,
  type SmartStatus,
  type Status,
} from "../bindings";
import { applyWindowMaterial } from "../appearance";
import { capitalize } from "../format";
import { TYPE_FILTERS } from "../launcher/filters";
import { FoldersSection } from "./FoldersSection";
import { SpacesSection } from "./SpacesSection";
import { TitleBar } from "./TitleBar";
import { StatusSection } from "./StatusSection";
import { Welcome } from "./Welcome";
import { Button } from "./ui";

/** Cómo escucha Sift el atajo, explicado (solo en «Avanzado»). */

type Page = "spaces" | "folders" | "general" | "welcome" | "status";

/** ¿Se abrió desde la barra con «Crear Espacio»? Entonces se empieza en Espacios. */
const startsWithNew = new URLSearchParams(window.location.search).has("new");

/** ¿Es la primera vez? Entonces se abre la bienvenida, sin navegación. */
const startsWelcome = new URLSearchParams(window.location.search).has(
  "welcome",
);

// El orden de las apps de Windows (Configuración, Terminal, PowerToys): lo general
// primero, después lo propio de la app, y la información («Estado») al final, abajo.
const PAGES: { id: Page; label: string }[] = [
  { id: "general", label: "General" },
  { id: "spaces", label: "Espacios" },
  { id: "folders", label: "Carpetas" },
];
const FOOTER_PAGES: { id: Page; label: string }[] = [
  { id: "welcome", label: "Primeros pasos" },
  { id: "status", label: "Estado" },
];

function Settings() {
  const [page, setPage] = useState<Page>(startsWithNew ? "spaces" : "general");
  const [status, setStatus] = useState<Status | null>(null);
  const [settings, setSettings] = useState<SettingsView | null>(null);
  // La bienvenida de la primera vez: mientras no se termine, no hay navegación.
  const [inWelcome, setInWelcome] = useState(startsWelcome);
  const [newSpace, setNewSpace] = useState(false);

  const refreshStatus = () => void commands.getStatus().then(setStatus);

  useEffect(() => {
    refreshStatus();
    void commands.getSettings().then(setSettings);
  }, []);

  const navButton = (p: { id: Page; label: string }) => (
    <button
      key={p.id}
      type="button"
      aria-current={page === p.id ? "page" : undefined}
      onClick={() => {
        setPage(p.id);
        if (p.id === "status") refreshStatus();
      }}
      className="sift-option sift-option-soft px-3 py-1.5 text-left text-sm outline-none hover:bg-hover focus-visible:ring-2 focus-visible:ring-accent-ink"
    >
      {p.label}
    </button>
  );

  if (inWelcome) {
    return (
      <div className="flex h-full flex-col bg-surface">
        <TitleBar title="Sift" />
        <main className="min-h-0 flex-1 select-text">
          <Welcome
            mode="first-run"
            onCreateSpace={() => {
              setNewSpace(true);
              setPage("spaces");
              setInWelcome(false);
            }}
          />
        </main>
      </div>
    );
  }

  return (
    <div className="flex h-full flex-col bg-surface">
      <TitleBar title="Ajustes de Sift" />
      <div className="flex min-h-0 flex-1">
        <nav
          aria-label="Secciones"
          className="flex w-44 shrink-0 flex-col gap-1 border-r border-line px-3 pb-3"
        >
          <h1 className="mb-3 px-2 text-xl font-semibold">Ajustes</h1>
          {PAGES.map(navButton)}
          <span className="flex-1" />
          {status?.storageError && (
            <p role="alert" className="px-2 text-xs text-danger">
              La base de datos no se pudo abrir: los cambios no se guardarán.
              Mira «Estado».
            </p>
          )}
          {FOOTER_PAGES.map(navButton)}
        </nav>

        <main className="min-w-0 flex-1 select-text">
          {page === "spaces" && <SpacesSection startNew={newSpace} />}
          {page === "welcome" && (
            <Welcome mode="settings" onGoTo={(p) => setPage(p)} />
          )}
          {page === "folders" && <FoldersSection />}
          {page === "general" && (
            <div className="h-full space-y-8 overflow-y-auto p-6">
              {settings ? (
                <>
                  <HotkeyForm
                    initial={settings.hotkey}
                    onSaved={refreshStatus}
                  />
                  <StartupToggle initial={settings.launchAtStartup} />
                  <IconsToggle initial={settings.iconsHidden} />
                  <GlassToggle initial={settings.disableGlass} />
                  <SmartSearchToggle />
                </>
              ) : (
                <p>Cargando…</p>
              )}
            </div>
          )}
          {page === "status" && (
            <div className="h-full overflow-y-auto p-6">
              <Section title="Estado">
                {status ? <StatusSection status={status} /> : <p>Cargando…</p>}
              </Section>
            </div>
          )}
        </main>
      </div>
    </div>
  );
}

/** «Búsqueda inteligente»: buscar por significado (ADR 0009). Activada por defecto. */
function SmartSearchToggle() {
  const [enabled, setEnabled] = useState<boolean | null>(null);
  const [smart, setSmart] = useState<SmartStatus | null>(null);
  useEffect(() => {
    void commands.getSmartSearch().then(setEnabled);
    void commands.indexStatus().then((s) => setSmart(s.smart));
    const unlisten = events.indexProgress.listen((e) =>
      setSmart(e.payload.smart),
    );
    return () => {
      void unlisten.then((stop) => stop());
    };
  }, []);
  return (
    <Section title="Búsqueda">
      <label className="flex items-start gap-3">
        <input
          type="checkbox"
          checked={enabled ?? true}
          disabled={enabled === null}
          onChange={(e) => {
            void commands.setSmartSearch(e.currentTarget.checked).then((r) => {
              if (r.status === "ok") setEnabled(r.data);
            });
          }}
          className="mt-1 size-4 accent-accent"
        />
        <span>
          Búsqueda inteligente
          <span className="block text-xs text-muted">
            Encuentra también lo que trata de lo mismo aunque use otras
            palabras: buscando «viaje» salen también las reservas de hotel o los
            billetes de tren. Funciona en tu PC, sin enviar nada. Prepara tus
            archivos poco a poco, cuando no estás usando mucho el ordenador.
          </span>
        </span>
      </label>
      {smart?.awaitingPermission && (
        <div className="space-y-2 pl-7">
          <p className="text-xs text-muted">
            Falta descargar el modelo (unos {smart.downloadMb} MB). Funciona en
            tu PC, sin enviar nada.
          </p>
          <Button
            variant="primary"
            onClick={() => void commands.allowModelDownload()}
          >
            Descargar ({smart.downloadMb} MB)
          </Button>
        </div>
      )}
      <QuickFilters />
    </Section>
  );
}

/** Cómo buscar solo un tipo de archivo, sacado de la misma lista que usa la barra. */
function QuickFilters() {
  // Las palabras, como se escriben (con tilde donde la lleva: «código», «vídeo»).
  const spoken: Record<string, string> = { codigo: "código", video: "vídeo" };
  return (
    <div className="space-y-2 pt-2">
      <h3 className="text-sm font-semibold">Buscar solo un tipo de archivo</h3>
      <p className="text-xs text-muted">
        Empieza con una de estas palabras y un espacio; por ejemplo, «pdf
        contrato» busca «contrato» solo en tus PDF. La palabra se queda como
        etiqueta junto a la caja; Retroceso la quita.
      </p>
      <ul className="grid grid-cols-[auto_1fr] gap-x-4 gap-y-1 text-sm">
        {TYPE_FILTERS.map((f) => (
          <li key={f.id} className="contents">
            <span className="font-mono text-xs leading-5">
              {f.keywords
                // Sin los plurales de otra palabra («fotos» si ya está «foto»).
                .filter(
                  (k) =>
                    !f.keywords.some((o) => k === `${o}s` || k === `${o}es`),
                )
                .map((k) => spoken[k] ?? k)
                .join(" · ")}
            </span>
            <span className="text-muted">{capitalize(f.plural)}</span>
          </li>
        ))}
      </ul>
    </div>
  );
}

function GlassToggle({ initial }: { initial: boolean }) {
  const [disabled, setDisabled] = useState(initial);
  return (
    <label className="flex items-start gap-3">
      <input
        type="checkbox"
        checked={disabled}
        onChange={(e) => {
          const next = e.currentTarget.checked;
          void commands.setGlassDisabled(next).then((r) => {
            if (r.status !== "ok") return;
            setDisabled(r.data);
            void applyWindowMaterial(); // esta ventana cambia al momento
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
      </span>
    </label>
  );
}

function IconsToggle({ initial }: { initial: boolean }) {
  const [hidden, setHidden] = useState(initial);
  return (
    <Section title="Apariencia">
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
    </Section>
  );
}

function Section({
  title,
  children,
}: {
  title: string;
  children: React.ReactNode;
}) {
  return (
    <section className="space-y-3">
      <h2 className="text-sm font-semibold">{title}</h2>
      {children}
    </section>
  );
}

function HotkeyForm({
  initial,
  onSaved,
}: {
  initial: string;
  onSaved: () => void;
}) {
  const [value, setValue] = useState(initial);
  const [saved, setSaved] = useState(initial);
  const [message, setMessage] = useState<{ ok: boolean; text: string } | null>(
    null,
  );

  async function save(e: React.FormEvent) {
    e.preventDefault();
    const result = await commands.setHotkey(value);
    if (result.status === "ok") {
      setValue(result.data);
      setSaved(result.data);
      setMessage({ ok: true, text: `Atajo cambiado a ${result.data}.` });
      onSaved();
    } else {
      setMessage({ ok: false, text: result.error.message });
    }
  }

  return (
    <Section title="Atajo para abrir Sift">
      <form onSubmit={(e) => void save(e)} className="flex gap-2">
        <input
          value={value}
          onChange={(e) => setValue(e.currentTarget.value)}
          aria-label="Atajo"
          aria-describedby="hotkey-help"
          spellCheck={false}
          className="sift-field flex-1 bg-hover px-3 py-2"
        />
        <button
          type="submit"
          disabled={value.trim() === saved}
          className="rounded-item bg-accent px-4 py-2 text-accent-fg disabled:opacity-40"
        >
          Guardar
        </button>
      </form>
      <p id="hotkey-help" className="text-xs text-muted">
        Una o varias de las teclas Ctrl, Alt, Shift o Win, más otra tecla,
        unidas con «+». Por ejemplo: Alt+Space, Ctrl+Shift+K o Win+F2.
      </p>
      {message && (
        <p
          role="status"
          className={`text-sm ${message.ok ? "text-success" : "text-danger"}`}
        >
          {message.text}
        </p>
      )}
    </Section>
  );
}

function StartupToggle({ initial }: { initial: boolean }) {
  const [enabled, setEnabled] = useState(initial);
  const [error, setError] = useState<string | null>(null);

  async function toggle(next: boolean) {
    const result = await commands.setLaunchAtStartup(next);
    if (result.status === "ok") {
      setEnabled(result.data);
      setError(null);
    } else {
      setError(result.error.message);
    }
  }

  return (
    <Section title="Inicio">
      <label className="flex items-center gap-3">
        <input
          type="checkbox"
          checked={enabled}
          onChange={(e) => void toggle(e.currentTarget.checked)}
          className="size-4 accent-accent"
        />
        Arrancar Sift al iniciar sesión en Windows
      </label>
      {error && (
        <p role="alert" className="text-sm text-danger">
          {error}
        </p>
      )}
    </Section>
  );
}

export default Settings;
