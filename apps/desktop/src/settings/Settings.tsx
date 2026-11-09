import { useEffect, useState } from "react";
import { commands, type SettingsView, type Status } from "../bindings";
import { SpacesSection } from "./SpacesSection";

const BACKEND_LABEL = {
  registerHotKey: "RegisterHotKey (vía principal)",
  keyboardHook: "Hook de teclado (vía alternativa)",
} as const;

type Page = "spaces" | "general" | "status";

const PAGES: { id: Page; label: string }[] = [
  { id: "spaces", label: "Espacios" },
  { id: "general", label: "General" },
  { id: "status", label: "Estado" },
];

function Settings() {
  const [page, setPage] = useState<Page>("spaces");
  const [status, setStatus] = useState<Status | null>(null);
  const [settings, setSettings] = useState<SettingsView | null>(null);

  const refreshStatus = () => void commands.getStatus().then(setStatus);

  useEffect(() => {
    refreshStatus();
    void commands.getSettings().then(setSettings);
  }, []);

  return (
    <div className="flex h-full bg-surface">
      <nav
        aria-label="Secciones"
        className="flex w-44 shrink-0 flex-col gap-1 border-r border-line p-3"
      >
        <h1 className="mb-3 px-2 text-lg font-semibold">Ajustes</h1>
        {PAGES.map((p) => (
          <button
            key={p.id}
            type="button"
            aria-current={page === p.id ? "page" : undefined}
            onClick={() => {
              setPage(p.id);
              if (p.id === "status") refreshStatus();
            }}
            className="sift-option px-3 py-1.5 text-left text-sm outline-none hover:bg-hover focus-visible:ring-2 focus-visible:ring-accent"
          >
            {p.label}
          </button>
        ))}
        {status?.storageError && (
          <p role="alert" className="mt-auto px-2 text-xs text-danger">
            La base de datos no se pudo abrir: los cambios no se guardarán. Mira
            «Estado».
          </p>
        )}
      </nav>

      <main className="min-w-0 flex-1 select-text">
        {page === "spaces" && <SpacesSection />}
        {page === "general" && (
          <div className="h-full space-y-8 overflow-y-auto p-6">
            {settings ? (
              <>
                <HotkeyForm initial={settings.hotkey} onSaved={refreshStatus} />
                <StartupToggle initial={settings.launchAtStartup} />
                <IconsToggle initial={settings.iconsHidden} />
              </>
            ) : (
              <p>Cargando…</p>
            )}
          </div>
        )}
        {page === "status" && (
          <div className="h-full overflow-y-auto p-6">
            <Section title="Estado">
              {status ? <StatusList status={status} /> : <p>Cargando…</p>}
            </Section>
          </div>
        )}
      </main>
    </div>
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
      <h2 className="text-sm font-semibold tracking-wide text-muted uppercase">
        {title}
      </h2>
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
          className="flex-1 rounded-item border border-line bg-hover px-3 py-2 outline-none focus:border-accent"
        />
        <button
          type="submit"
          disabled={value.trim() === saved}
          className="rounded-item bg-accent px-4 py-2 font-medium text-accent-fg disabled:opacity-40"
        >
          Guardar
        </button>
      </form>
      <p id="hotkey-help" className="text-xs text-muted">
        Modificadores (Ctrl, Alt, Shift, Win) + una tecla, separados por «+».
        Ejemplos: Alt+Space, Ctrl+Shift+K, Win+F2.
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

function StatusList({ status }: { status: Status }) {
  const l = status.latency;
  return (
    <dl className="grid grid-cols-[auto_1fr] gap-x-6 gap-y-1 text-sm">
      <dt className="text-muted">Atajo</dt>
      <dd>{status.hotkey ?? "ninguno (no se pudo registrar)"}</dd>
      <dt className="text-muted">Mecanismo</dt>
      <dd>{status.backend ? BACKEND_LABEL[status.backend] : "—"}</dd>
      <dt className="text-muted">Acrylic</dt>
      <dd>{status.acrylic ? "activo" : "no disponible (fondo opaco)"}</dd>
      <dt className="text-muted">Base de datos</dt>
      <dd>
        {status.storageError
          ? `no disponible: ${status.storageError}`
          : "correcta"}
      </dd>
      <dt className="text-muted">Latencia</dt>
      <dd>
        {l
          ? `última ${l.lastMs} ms · p50 ${l.p50Ms} ms · p95 ${l.p95Ms} ms (${l.samples} medidas)`
          : "aún sin medidas: abre el lanzador con el atajo"}
      </dd>
    </dl>
  );
}

export default Settings;
