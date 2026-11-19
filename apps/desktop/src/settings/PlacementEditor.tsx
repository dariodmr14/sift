// «Colocar su ventana» en una acción que abre algo (Fase 6.5, ADR 0014): miniaturas de
// pantalla con cada zona pintada, por filas con nombre (General, Mitades, Tercios,
// Cuartos), en qué pantalla y si usar la ventana que ya esté abierta.

import { useEffect, useState } from "react";
import {
  commands,
  type MonitorChoice,
  type MonitorInfo,
  type Zone,
} from "../bindings";
import { Glyph } from "../components/Glyph";
import { Button, Field, Select } from "./ui";
import {
  monitorOptions,
  placementSummary,
  sameChoice,
  ZONE_GROUPS,
  ZONES,
} from "./zones";

export interface PlacementValue {
  /** `null` = no colocarla (pero sí usar la ya abierta, si `reuseWindow`). */
  zone: Zone | null;
  monitor: MonitorChoice;
  reuseWindow: boolean;
}

/** Sin zona ni «usar la ya abierta», no hay nada que guardar: la acción va sin `place`. */
export function normalizePlacement(
  value: PlacementValue,
): PlacementValue | null {
  return value.zone === null && !value.reuseWindow ? null : value;
}

/** Una miniatura de pantalla con la zona en el color de acento («No colocar»: vacía). */
function ZoneThumb({
  sixths,
}: {
  sixths: [number, number, number, number] | null;
}) {
  const pct = (n: number) => `${(n / 6) * 100}%`;
  return (
    <span className="relative flex h-7 w-11 items-center justify-center overflow-hidden rounded-[3px] border border-line bg-hover">
      {sixths ? (
        <span
          className="absolute rounded-[1px] bg-accent"
          style={{
            left: pct(sixths[0]),
            top: pct(sixths[1]),
            width: pct(sixths[2]),
            height: pct(sixths[3]),
          }}
        />
      ) : (
        <Glyph name="dash" className="text-xs text-muted" />
      )}
    </span>
  );
}

const NO_PLACE_LABEL = "No colocar: donde la ponga Windows";

export function PlacementEditor({
  value,
  onChange,
}: {
  value: PlacementValue | null | undefined;
  onChange: (value: PlacementValue | null) => void;
}) {
  const [monitors, setMonitors] = useState<MonitorInfo[]>([]);
  // Plegado si ya hay una posición elegida: una línea con lo elegido y «Cambiar».
  const [open, setOpen] = useState(!value?.zone);
  useEffect(() => {
    let alive = true;
    void commands.listMonitors().then((r) => {
      if (alive && r.status === "ok") setMonitors(r.data);
    });
    return () => {
      alive = false;
    };
  }, []);

  const current: PlacementValue = value ?? {
    zone: null,
    monitor: { kind: "primary" },
    reuseWindow: false,
  };
  const options = monitorOptions(monitors, current.monitor);
  const selectedMonitor =
    options.find((o) => sameChoice(o.choice, current.monitor))?.value ??
    "primary";
  const set = (next: Partial<PlacementValue>) =>
    onChange(normalizePlacement({ ...current, ...next }));

  const summary = placementSummary(current, options);
  if (!open)
    return (
      <div className="flex items-center gap-3">
        <div className="min-w-0 flex-1">
          <span className="block text-xs text-muted">Colocar su ventana</span>
          <p className="truncate text-sm">
            {current.zone ? `Posición: ${summary}` : summary}
          </p>
        </div>
        <Button
          onClick={() => setOpen(true)}
          aria-label={`Cambiar dónde colocar su ventana (${summary})`}
        >
          Cambiar
        </Button>
      </div>
    );

  return (
    <div className="space-y-3">
      <div className="flex items-center gap-3">
        <span className="flex-1 text-xs text-muted">Colocar su ventana</span>
        {current.zone && (
          <Button variant="ghost" onClick={() => setOpen(false)}>
            Listo
          </Button>
        )}
      </div>
      <div
        role="radiogroup"
        aria-label="Dónde colocar su ventana"
        className="grid grid-cols-[auto_1fr] items-center gap-x-4 gap-y-1"
      >
        {ZONE_GROUPS.map((group) => (
          <div key={group.label} className="contents">
            <span aria-hidden className="text-xs text-muted">
              {group.label}
            </span>
            <div className="flex flex-wrap gap-1">
              {group.zones.map((id) => {
                const zone = id ? ZONES.find((z) => z.id === id) : null;
                const label = zone?.label ?? NO_PLACE_LABEL;
                const checked = current.zone === id;
                return (
                  <button
                    key={id ?? "none"}
                    type="button"
                    role="radio"
                    aria-checked={checked}
                    aria-label={label}
                    title={label}
                    onClick={() => set({ zone: id })}
                    className={`rounded-item p-1 outline-none focus-visible:ring-2 focus-visible:ring-accent-ink ${
                      checked ? "bg-selected" : "hover:bg-hover"
                    }`}
                  >
                    <ZoneThumb sixths={zone?.sixths ?? null} />
                  </button>
                );
              })}
            </div>
          </div>
        ))}
      </div>
      <p className="text-sm font-semibold" aria-live="polite">
        Posición seleccionada:{" "}
        {current.zone
          ? ZONES.find((z) => z.id === current.zone)?.label
          : NO_PLACE_LABEL}
      </p>
      {current.zone && (
        <div className="flex items-end gap-2">
          <div className="w-full max-w-sm">
            <Field label="Pantalla">
              <Select
                value={selectedMonitor}
                onChange={(e) => {
                  const option = options.find(
                    (o) => o.value === e.currentTarget.value,
                  );
                  if (option) set({ monitor: option.choice });
                }}
                className="w-full"
              >
                {options.map((o) => (
                  <option key={o.value} value={o.value}>
                    {o.label}
                  </option>
                ))}
              </Select>
            </Field>
          </div>
          <Button
            onClick={() => void commands.identifyMonitors()}
            title="Enseña unos segundos el número de cada pantalla"
          >
            Identificar
          </Button>
        </div>
      )}
      <label className="flex items-center gap-2 text-sm">
        <input
          type="checkbox"
          checked={current.reuseWindow}
          onChange={(e) => set({ reuseWindow: e.currentTarget.checked })}
          className="size-4 accent-accent"
        />
        Si ya está abierta, usar esa en vez de abrir otra
      </label>
    </div>
  );
}
