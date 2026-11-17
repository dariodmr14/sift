// «Colocar su ventana» en una acción que abre algo (Fase 6.5, ADR 0014): una rejilla de
// miniaturas de pantalla con cada zona pintada, en qué pantalla y si mover la que ya
// esté abierta.

import { useEffect, useState } from "react";
import {
  commands,
  type MonitorChoice,
  type MonitorInfo,
  type Zone,
} from "../bindings";
import { Select } from "./ui";
import { monitorOptions, sameChoice, ZONE_ROWS, ZONES } from "./zones";

export interface PlacementValue {
  zone: Zone;
  monitor: MonitorChoice;
  reuseWindow: boolean;
}

/** Una miniatura de pantalla con la zona en el color de acento. */
function ZoneThumb({
  sixths,
}: {
  sixths: [number, number, number, number] | null;
}) {
  const pct = (n: number) => `${(n / 6) * 100}%`;
  return (
    <span className="relative block h-7 w-11 overflow-hidden rounded-[3px] border border-line bg-hover">
      {sixths && (
        <span
          className="absolute rounded-[1px] bg-accent"
          style={{
            left: pct(sixths[0]),
            top: pct(sixths[1]),
            width: pct(sixths[2]),
            height: pct(sixths[3]),
          }}
        />
      )}
    </span>
  );
}

export function PlacementEditor({
  value,
  onChange,
}: {
  value: PlacementValue | null | undefined;
  onChange: (value: PlacementValue | null) => void;
}) {
  const [monitors, setMonitors] = useState<MonitorInfo[]>([]);
  useEffect(() => {
    let alive = true;
    void commands.listMonitors().then((r) => {
      if (alive && r.status === "ok") setMonitors(r.data);
    });
    return () => {
      alive = false;
    };
  }, []);

  const current = value ?? null;
  const options = monitorOptions(
    monitors,
    current?.monitor ?? { kind: "primary" },
  );
  const selectedMonitor =
    options.find((o) => current && sameChoice(o.choice, current.monitor))
      ?.value ?? "primary";

  const pick = (zone: Zone | null) =>
    onChange(
      zone === null
        ? null
        : {
            zone,
            monitor: current?.monitor ?? { kind: "primary" },
            reuseWindow: current?.reuseWindow ?? false,
          },
    );

  return (
    <div className="space-y-2">
      <span className="block text-xs text-muted">Colocar su ventana</span>
      {/* Por filas con sentido (completa y mitades · tercios · cuartos), no como caigan. */}
      <div
        role="radiogroup"
        aria-label="Dónde colocar su ventana"
        className="flex items-start gap-3"
      >
        <button
          type="button"
          role="radio"
          aria-checked={current === null}
          aria-label="No colocar: donde la ponga Windows"
          title="No colocar: donde la ponga Windows"
          onClick={() => pick(null)}
          className={`flex h-9 shrink-0 items-center rounded-item px-2 text-xs outline-none focus-visible:ring-2 focus-visible:ring-accent-ink ${
            current === null ? "bg-selected" : "hover:bg-hover"
          }`}
        >
          No colocar
        </button>
        <div className="space-y-1">
          {ZONE_ROWS.map((row) => (
            <div key={row[0]} className="flex gap-1">
              {row.map((id) => {
                const z = ZONES.find((zone) => zone.id === id)!;
                return (
                  <button
                    key={z.id}
                    type="button"
                    role="radio"
                    aria-checked={current?.zone === z.id}
                    aria-label={z.label}
                    title={z.label}
                    onClick={() => pick(z.id)}
                    className={`rounded-item p-1 outline-none focus-visible:ring-2 focus-visible:ring-accent-ink ${
                      current?.zone === z.id ? "bg-selected" : "hover:bg-hover"
                    }`}
                  >
                    <ZoneThumb sixths={z.sixths} />
                  </button>
                );
              })}
            </div>
          ))}
        </div>
      </div>
      {current && (
        <div className="flex flex-wrap items-center gap-x-3 gap-y-2 text-sm">
          <span>{ZONES.find((z) => z.id === current.zone)?.label} de</span>
          <Select
            aria-label="En qué pantalla"
            value={selectedMonitor}
            onChange={(e) => {
              const option = options.find(
                (o) => o.value === e.currentTarget.value,
              );
              if (option) onChange({ ...current, monitor: option.choice });
            }}
          >
            {options.map((o) => (
              <option key={o.value} value={o.value}>
                {o.label}
              </option>
            ))}
          </Select>
          <label className="flex items-center gap-2">
            <input
              type="checkbox"
              checked={current.reuseWindow}
              onChange={(e) =>
                onChange({ ...current, reuseWindow: e.currentTarget.checked })
              }
              className="size-4 accent-accent"
            />
            Si ya está abierta, moverla en vez de abrir otra
          </label>
        </div>
      )}
    </div>
  );
}
