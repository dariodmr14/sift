import { describe, expect, it } from "vitest";
import { commandLine, spaceSteps } from "./spacePreview";

describe("vista previa de un Espacio", () => {
  it("describe cada acción en orden, con dónde se coloca", () => {
    const steps = spaceSteps([
      {
        kind: "openFolder",
        path: "C:\\Users\\Ana\\Proyectos",
        place: {
          zone: "leftHalf",
          monitor: { kind: "underMouse" },
          reuseWindow: true,
        },
      },
      {
        kind: "runCommand",
        program: "docker",
        args: ["compose", "up"],
        cwd: "C:\\app",
      },
      {
        kind: "wait",
        until: { kind: "webResponds", url: "http://localhost:8080" },
        timeoutSecs: 60,
        onTimeout: "stop",
      },
      { kind: "openUrl", url: "http://localhost:8080" },
      {
        kind: "wait",
        until: { kind: "seconds", secs: 5 },
        timeoutSecs: 60,
        onTimeout: "stop",
      },
    ]);
    expect(steps.map((s) => s.title)).toEqual([
      "Abrir la carpeta «Proyectos»",
      "Ejecutar docker compose up",
      "Esperar a que responda http://localhost:8080",
      "Abrir http://localhost:8080",
      "Esperar 5 s",
    ]);
    expect(steps[0].details).toEqual([
      "C:\\Users\\Ana\\Proyectos",
      "Colocar en: Mitad izquierda, en la pantalla donde esté el ratón",
      "Si ya está abierta, usa esa en vez de abrir otra",
    ]);
    expect(steps[1].details).toEqual(["En C:\\app"]);
    expect(steps[2].details).toEqual(["Si no ocurre en 60 s, se para aquí"]);
    expect(steps[3].details).toEqual([]);
    expect(steps[4].details).toEqual([]);
  });

  it("una pantalla concreta se nombra", () => {
    const [s] = spaceSteps([
      {
        kind: "openApp",
        appId: "x",
        name: "Bloc de notas",
        place: {
          zone: "maximized",
          monitor: { kind: "specific", name: "DELL U2720Q", left: 0, top: 0 },
          reuseWindow: false,
        },
      },
    ]);
    expect(s.details).toEqual(["Colocar en: Toda la pantalla, en DELL U2720Q"]);
  });

  it("los argumentos con espacios van entre comillas", () => {
    expect(commandLine("code", ["C:\\Mis cosas", ""])).toBe(
      'code "C:\\Mis cosas" ""',
    );
  });
});
