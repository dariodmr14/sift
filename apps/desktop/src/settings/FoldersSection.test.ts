import { describe, expect, it } from "vitest";
import type { IndexStatus } from "../bindings";
import { files, folderName, shortParent, since, statusLine } from "./folders";

describe("since", () => {
  const now = Date.parse("2026-11-10T12:00:00Z");
  it("dice cuánto hace en palabras", () => {
    expect(since(null, now)).toBe("aún no");
    expect(since("2026-11-10T11:59:30Z", now)).toBe("hace un momento");
    expect(since("2026-11-10T11:45:00Z", now)).toBe("hace 15 min");
    expect(since("2026-11-10T09:00:00Z", now)).toBe("hace 3 h");
    expect(since("2026-11-08T12:00:00Z", now)).toBe("hace 2 d");
  });
});

describe("carpetas", () => {
  it("separa el nombre y abrevia dónde está", () => {
    const p = "C:\\Users\\Ana\\Documents\\GitHub\\Sift";
    expect(folderName(p)).toBe("Sift");
    expect(shortParent(p)).toBe("C:\\…\\Documents\\GitHub");
    expect(shortParent("D:\\Apuntes\\Uni")).toBe("D:\\Apuntes");
  });

  it("cuenta archivos con punto de miles y en singular", () => {
    expect(files(1240)).toBe("1.240 archivos");
    expect(files(1)).toBe("1 archivo");
  });
});

describe("statusLine", () => {
  const base: IndexStatus = {
    running: false,
    phase: "idle",
    scanned: 0,
    done: 0,
    total: 0,
  };

  it("al día, con el total de archivos", () => {
    expect(statusLine(base, 1240, 2)).toEqual({
      text: "Al día · 1.240 archivos",
      progress: null,
    });
  });

  it("buscando cambios, sin total todavía", () => {
    const s = { ...base, running: true, phase: "scanning" as const };
    expect(statusLine(s, 0, 1)).toEqual({
      text: "Buscando cambios…",
      progress: "indeterminate",
    });
  });

  it("leyendo X de Y, con la barra en su punto", () => {
    const s = {
      ...base,
      running: true,
      phase: "reading" as const,
      done: 31,
      total: 120,
    };
    const line = statusLine(s, 0, 1);
    expect(line.text).toBe("Leyendo 32 de 120 archivos");
    expect(line.progress).toBeCloseTo(31 / 120);
  });

  it("sin carpetas, invita a añadir una", () => {
    expect(statusLine(base, 0, 0).text).toBe("Añade una carpeta para empezar.");
  });
});
