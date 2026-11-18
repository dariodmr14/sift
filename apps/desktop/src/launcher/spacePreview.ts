// La vista previa de un Espacio: sus acciones en orden, en lenguaje normal (qué abrirá,
// qué esperará y dónde colocará cada ventana). Lógica pura, para probarla.

import type { MonitorChoice, Placement_Deserialize } from "../bindings";
import { ZONES } from "../settings/zones";
import type { SpaceView } from "./mode";

type Action = SpaceView["space"]["actions"][number];

export interface SpaceStep {
  /** Qué hace («Abrir la carpeta «Proyectos»»). */
  title: string;
  /** Lo demás, una línea por cosa (dónde, cuánto espera, qué hace si no llega). */
  details: string[];
}

/** El último trozo de una ruta, con barras de Windows o normales. */
function baseName(path: string): string {
  const parts = path.split(/[\\/]+/).filter(Boolean);
  return parts[parts.length - 1] ?? path;
}

/** Un comando tal cual se ejecutará: cada argumento con comillas si tiene espacios. */
export function commandLine(program: string, args: string[]): string {
  const quote = (s: string) => (s === "" || /\s/.test(s) ? `"${s}"` : s);
  return [program, ...args].map(quote).join(" ");
}

function monitorText(m: MonitorChoice | undefined): string {
  if (!m || m.kind === "primary") return "en la pantalla principal";
  if (m.kind === "underMouse") return "en la pantalla donde esté el ratón";
  return `en ${m.name}`;
}

function placementDetails(place: Placement_Deserialize | null | undefined) {
  if (!place) return [];
  const lines: string[] = [];
  if (place.zone) {
    const zone = ZONES.find((z) => z.id === place.zone)?.label ?? place.zone;
    lines.push(`Colocar en: ${zone}, ${monitorText(place.monitor)}`);
  }
  if (place.reuseWindow)
    lines.push("Si ya está abierta, usa esa en vez de abrir otra");
  return lines;
}

function waitDetails(a: Extract<Action, { kind: "wait" }>): string[] {
  if (a.until.kind === "seconds") return [];
  const secs = a.timeoutSecs ?? 60;
  return [
    a.onTimeout === "continue"
      ? `Si no ocurre en ${secs} s, sigue con lo demás`
      : `Si no ocurre en ${secs} s, se para aquí`,
  ];
}

export function spaceStep(a: Action): SpaceStep {
  switch (a.kind) {
    case "openFolder":
      return {
        title: `Abrir la carpeta «${baseName(a.path)}»`,
        details: [a.path, ...placementDetails(a.place)],
      };
    case "openFile":
      return {
        title: `Abrir «${baseName(a.path)}»`,
        details: [a.path, ...placementDetails(a.place)],
      };
    case "openUrl":
      return {
        title: `Abrir ${a.url}`,
        details: placementDetails(a.place),
      };
    case "openApp":
      return {
        title: `Abrir ${a.name}`,
        details: placementDetails(a.place),
      };
    case "runCommand":
      return {
        title: `Ejecutar ${commandLine(a.program, a.args ?? [])}`,
        details: a.cwd ? [`En ${a.cwd}`] : [],
      };
    case "wait": {
      const u = a.until;
      const title =
        u.kind === "webResponds"
          ? `Esperar a que responda ${u.url}`
          : u.kind === "serviceAccepts"
            ? `Esperar a que el puerto ${u.port} acepte conexiones`
            : u.kind === "commandFinished"
              ? "Esperar a que termine el comando anterior"
              : `Esperar ${u.secs} s`;
      return { title, details: waitDetails(a) };
    }
  }
}

export function spaceSteps(actions: Action[]): SpaceStep[] {
  return actions.map(spaceStep);
}
