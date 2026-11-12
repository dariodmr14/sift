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
  withWaitKind,
  type WaitAction,
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

describe("esperas", () => {
  const wait = newAction("wait") as WaitAction;

  it("una espera nueva: web de tu equipo, 60 s y no seguir si no responde", () => {
    expect(wait).toEqual({
      kind: "wait",
      until: { kind: "webResponds", url: "http://localhost:" },
      timeoutSecs: 60,
      onTimeout: "stop",
    });
  });

  it("al cambiar el tipo, el tiempo por defecto cambia con él", () => {
    const cmd = withWaitKind(wait, "commandFinished");
    expect(cmd.until).toEqual({ kind: "commandFinished" });
    expect(cmd.timeoutSecs).toBe(120);
    expect(withWaitKind(cmd, "serviceAccepts").timeoutSecs).toBe(60);
  });

  it("si cambiaste el tiempo, se respeta", () => {
    const custom = { ...wait, timeoutSecs: 300 };
    expect(withWaitKind(custom, "commandFinished").timeoutSecs).toBe(300);
  });

  it("cada tipo empieza con sus valores", () => {
    expect(withWaitKind(wait, "seconds").until).toEqual({
      kind: "seconds",
      secs: 5,
    });
    expect(withWaitKind(wait, "serviceAccepts").until).toEqual({
      kind: "serviceAccepts",
      port: 0,
    });
  });
});
