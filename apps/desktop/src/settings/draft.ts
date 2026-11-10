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

export const ACTION_LABELS: Record<ActionKind, string> = {
  openFolder: "Abrir carpeta",
  openFile: "Abrir archivo",
  openUrl: "Abrir web",
  openApp: "Abrir aplicación",
  runCommand: "Ejecutar comando",
};

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
