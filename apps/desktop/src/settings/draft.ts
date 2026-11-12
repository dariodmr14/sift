// Ayudantes puros del editor de Espacios (sin React, para poder probarlos).

import type { commands } from "../bindings";

type Ok<T> = Extract<T, { status: "ok" }> extends { data: infer D } ? D : never;
export type SpaceView = Ok<
  Awaited<ReturnType<typeof commands.listSpaces>>
>[number];
export type Space = SpaceView["space"];
/** Lo que se edita: un Espacio sin id. */
export type Draft = Omit<Space, "id">;
export type Action = Draft["actions"][number];
export type ActionKind = Action["kind"];
export type Icon = Draft["icon"];
export type WaitAction = Extract<Action, { kind: "wait" }>;
export type WaitKind = WaitAction["until"]["kind"];

export const ACTION_LABELS: Record<ActionKind, string> = {
  openFolder: "Abrir carpeta",
  openFile: "Abrir archivo",
  openUrl: "Abrir web",
  openApp: "Abrir aplicación",
  runCommand: "Ejecutar comando",
  wait: "Esperar…",
};

/** «Esperar a que…» + esto, en lenguaje normal (el desplegable del editor). */
export const WAIT_LABELS: Record<WaitKind, string> = {
  webResponds: "responda una web de tu equipo",
  serviceAccepts:
    "un servicio de tu equipo acepte conexiones (como una base de datos)",
  commandFinished: "termine el comando anterior",
  seconds: "pasen unos segundos",
};

/** Tiempo límite por defecto: un comando (instalar, arrancar contenedores) tarda más. */
export const DEFAULT_TIMEOUT: Record<WaitKind, number> = {
  webResponds: 60,
  serviceAccepts: 60,
  commandFinished: 120,
  seconds: 60,
};

/**
 * Cambia a qué se espera. Si el tiempo límite seguía siendo el de antes por defecto, pasa
 * al del tipo nuevo; si lo habías cambiado tú, se respeta.
 */
export function withWaitKind(action: WaitAction, kind: WaitKind): WaitAction {
  const untouched = action.timeoutSecs === DEFAULT_TIMEOUT[action.until.kind];
  const until: WaitAction["until"] =
    kind === "webResponds"
      ? { kind, url: "http://localhost:" }
      : kind === "serviceAccepts"
        ? { kind, port: 0 }
        : kind === "seconds"
          ? { kind, secs: 5 }
          : { kind };
  return {
    ...action,
    until,
    timeoutSecs: untouched ? DEFAULT_TIMEOUT[kind] : action.timeoutSecs,
  };
}

export function emptyDraft(): Draft {
  return { name: "", aliases: [], icon: { kind: "auto" }, actions: [] };
}

export function toDraft(space: Space): Draft {
  const { id: _id, ...draft } = space;
  return structuredClone(draft);
}

export function newAction(kind: ActionKind): Action {
  switch (kind) {
    case "openFolder":
    case "openFile":
      return { kind, path: "" };
    case "openUrl":
      return { kind, url: "https://" };
    case "openApp":
      return { kind, appId: "", name: "" };
    case "runCommand":
      return { kind, program: "", args: [], cwd: null };
    case "wait":
      return {
        kind,
        until: { kind: "webResponds", url: "http://localhost:" },
        timeoutSecs: DEFAULT_TIMEOUT.webResponds,
        onTimeout: "stop",
      };
  }
}

/** "bd, SQL ,, bases" → ["bd", "SQL", "bases"]. */
export function parseAliases(text: string): string[] {
  return text
    .split(",")
    .map((a) => a.trim())
    .filter(Boolean);
}

/**
 * Argumentos de un comando: **uno por línea**. Así no hay que pensar en comillas ni en
 * espacios: lo que hay en cada línea llega al programa tal cual, como un argumento.
 */
export function argsFromText(text: string): string[] {
  return text
    .split(/\r?\n/)
    .filter((line, i, all) => line !== "" || i < all.length - 1);
}

export function argsToText(args: string[]): string {
  return args.join("\n");
}

/** Mueve el elemento `index` una posición (`delta` = -1 arriba, +1 abajo). */
export function move<T>(list: T[], index: number, delta: -1 | 1): T[] {
  const to = index + delta;
  if (to < 0 || to >= list.length) return list;
  const copy = [...list];
  [copy[index], copy[to]] = [copy[to], copy[index]];
  return copy;
}

/** ¿Ha cambiado algo respecto a lo guardado? */
export function isDirty(draft: Draft, saved: Draft | null): boolean {
  return saved === null || JSON.stringify(draft) !== JSON.stringify(saved);
}
