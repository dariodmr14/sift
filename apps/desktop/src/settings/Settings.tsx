import { useEffect, useState } from "react";
import { commands, type SettingsView, type Status } from "../bindings";

const BACKEND_LABEL = {
  registerHotKey: "RegisterHotKey (vía principal)",
  keyboardHook: "Hook de teclado (vía alternativa)",
} as const;

function Settings() {
  const [status, setStatus] = useState<Status | null>(null);
  const [settings, setSettings] = useState<SettingsView | null>(null);

  const refreshStatus = () => void commands.getStatus().then(setStatus);

  useEffect(() => {
    refreshStatus();
    void commands.getSettings().then(setSettings);
  }, []);

  return (
    <main className="h-full space-y-8 overflow-y-auto bg-(--surface) p-6 select-text">
      <h1 className="text-2xl font-semibold">Ajustes</h1>

      {settings ? (
        <>
          <HotkeyForm initial={settings.hotkey} onSaved={refreshStatus} />
          <StartupToggle initial={settings.launchAtStartup} />
        </>
      ) : (
        <p>Cargando…</p>
      )}

      <Section title="Estado">
        {status ? <StatusList status={status} /> : <p>Cargando…</p>}
      </Section>
    </main>
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
      <h2 className="text-sm font-semibold tracking-wide text-(--text-muted) uppercase">
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
          className="flex-1 rounded-md border border-(--border) bg-(--surface-hover) px-3 py-2 outline-none focus:border-(--accent)"
        />
        <button
          type="submit"
          disabled={value.trim() === saved}
          className="rounded-md bg-(--accent) px-4 py-2 font-medium text-black disabled:opacity-40"
        >
          Guardar
        </button>
      </form>
      <p id="hotkey-help" className="text-xs text-(--text-muted)">
        Modificadores (Ctrl, Alt, Shift, Win) + una tecla, separados por «+».
        Ejemplos: Alt+Space, Ctrl+Shift+K, Win+F2.
      </p>
      {message && (
        <p
          role="status"
          className={`text-sm ${message.ok ? "text-green-400" : "text-red-400"}`}
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
          className="size-4 accent-(--accent)"
        />
        Arrancar Sift al iniciar sesión en Windows
      </label>
      {error && (
        <p role="alert" className="text-sm text-red-400">
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
      <dt className="text-(--text-muted)">Atajo</dt>
      <dd>{status.hotkey ?? "ninguno (no se pudo registrar)"}</dd>
      <dt className="text-(--text-muted)">Mecanismo</dt>
      <dd>{status.backend ? BACKEND_LABEL[status.backend] : "—"}</dd>
      <dt className="text-(--text-muted)">Acrylic</dt>
      <dd>{status.acrylic ? "activo" : "no disponible (fondo opaco)"}</dd>
      <dt className="text-(--text-muted)">Latencia</dt>
      <dd>
        {l
          ? `última ${l.lastMs} ms · p50 ${l.p50Ms} ms · p95 ${l.p95Ms} ms (${l.samples} medidas)`
          : "aún sin medidas: abre el lanzador con el atajo"}
      </dd>
    </dl>
  );
}

export default Settings;
