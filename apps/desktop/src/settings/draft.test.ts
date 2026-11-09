import { describe, expect, it } from "vitest";
import {
  argsFromText,
  argsToText,
  emptyDraft,
  isDirty,
  move,
  newAction,
  parseAliases,
  toDraft,
} from "./draft";

describe("parseAliases", () => {
  it("separa por comas y quita vacíos", () => {
    expect(parseAliases(" bd, SQL ,, bases ")).toEqual(["bd", "SQL", "bases"]);
    expect(parseAliases("")).toEqual([]);
  });
});

describe("argumentos de comando", () => {
  it("un argumento por línea, con espacios y comillas tal cual", () => {
    expect(argsFromText('compose\nup\nmi carpeta\n"x"')).toEqual([
      "compose",
      "up",
      "mi carpeta",
      '"x"',
    ]);
  });

  it("ignora la línea vacía final pero conserva argumentos vacíos intermedios", () => {
    expect(argsFromText("a\n\nb\n")).toEqual(["a", "", "b"]);
    expect(argsFromText("")).toEqual([]);
  });

  it("acepta saltos de línea de Windows", () => {
    expect(argsFromText("a\r\nb")).toEqual(["a", "b"]);
  });

  it("ida y vuelta", () => {
    const args = ["compose", "up", "-d"];
    expect(argsFromText(argsToText(args))).toEqual(args);
  });
});

describe("move", () => {
  it("sube y baja", () => {
    expect(move(["a", "b", "c"], 1, -1)).toEqual(["b", "a", "c"]);
    expect(move(["a", "b", "c"], 1, 1)).toEqual(["a", "c", "b"]);
  });

  it("no se sale de los bordes", () => {
    const list = ["a", "b"];
    expect(move(list, 0, -1)).toBe(list);
    expect(move(list, 1, 1)).toBe(list);
  });
});

describe("borradores", () => {
  it("toDraft quita el id y copia en profundidad", () => {
    const space = {
      id: 7,
      ...emptyDraft(),
      name: "X",
      actions: [newAction("openUrl")],
    };
    const draft = toDraft(space);
    expect("id" in draft).toBe(false);
    draft.actions.pop();
    expect(space.actions).toHaveLength(1);
  });

  it("isDirty detecta cambios", () => {
    const saved = { ...emptyDraft(), name: "X" };
    expect(isDirty({ ...saved }, saved)).toBe(false);
    expect(isDirty({ ...saved, name: "Y" }, saved)).toBe(true);
    expect(isDirty(saved, null)).toBe(true); // nuevo, sin guardar
  });

  it("newAction crea cada tipo con sus campos", () => {
    expect(newAction("runCommand")).toEqual({
      kind: "runCommand",
      program: "",
      args: [],
      cwd: null,
    });
    expect(newAction("openApp")).toEqual({
      kind: "openApp",
      appId: "",
      name: "",
    });
  });
});
