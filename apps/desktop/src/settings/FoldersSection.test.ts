import { describe, expect, it } from "vitest";
import type { IndexStatus } from "../bindings";
import { duration, files, since } from "../format";
import { folderName, shortParent, statusLine, statusNotes } from "./folders";

describe("since", () => {
  const now = Date.parse("2026-11-10T12:00:00Z");
  it("dice cuánto hace en palabras", () => {
    expect(since(null, now)).toBe("aún no");
    expect(since("2026-11-10T11:59:30Z", now)).toBe("hace un momento");
    expect(since("2026-11-10T11:45:00Z", now)).toBe("hace 15 min");
    expect(since("2026-11-10T09:00:00Z", now)).toBe("hace 3 h");
    expect(since("2026-11-10T11:00:30Z", now)).toBe("hace 59 min");
    expect(since("2026-11-09T11:00:00Z", now)).toBe("ayer");
    expect(since("2026-11-08T12:00:00Z", now)).toBe("hace 2 días");
  });
});

describe("duration", () => {
  it("usa minutos y horas, nunca cientos de segundos", () => {
    expect(duration(0.4)).toBe("menos de un segundo");
    expect(duration(35)).toBe("35 s");
    expect(duration(335)).toBe("5 min 35 s");
    expect(duration(120)).toBe("2 min");
    expect(duration(3900)).toBe("1 h 5 min");
    expect(duration(7200)).toBe("2 h");
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
    watching: true,
    readsPdf: true,
    smart: {
      enabled: true,
      ready: true,
      done: 0,
      total: 0,
      downloadedMb: 0,
      downloadMb: 0,
      awaitingPermission: false,
    },
  };

  it("descargando el modelo, en MB", () => {
    const s = {
      ...base,
      running: true,
      phase: "downloading" as const,
      smart: { ...base.smart, ready: false, downloadedMb: 45, downloadMb: 123 },
    };
    const line = statusLine(s, 0, 1);
    expect(line.text).toBe(
      "Descargando la búsqueda inteligente · 45 de 123 MB",
    );
    expect(line.progress).toBeCloseTo(45 / 123);
  });

  it("preparando la búsqueda inteligente, con su progreso", () => {
    const s = {
      ...base,
      running: true,
      phase: "embedding" as const,
      done: 3200,
      total: 13000,
    };
    expect(statusLine(s, 0, 1).text).toBe(
      "Preparando búsqueda inteligente · 3.200 de 13.000",
    );
  });

  it("explica por qué la búsqueda inteligente no está lista", () => {
    const notes = statusNotes({
      ...base,
      smart: {
        ...base.smart,
        ready: false,
        note: "Sin conexión: se descargará cuando la haya.",
      },
    });
    expect(notes).toEqual([
      "Búsqueda inteligente: Sin conexión: se descargará cuando la haya.",
    ]);
    const halfway = statusNotes({
      ...base,
      smart: { ...base.smart, done: 10, total: 30 },
    });
    expect(halfway[0]).toContain("10 de 30");
  });

  it("en pausa si el PC está ocupado, sin perder la cuenta", () => {
    const s = {
      ...base,
      running: true,
      phase: "paused" as const,
      done: 32,
      total: 120,
    };
    const line = statusLine(s, 0, 1);
    expect(line.text).toBe("En pausa mientras usas el PC · 32 de 120 archivos");
    expect(line.progress).toBeCloseTo(32 / 120);
  });

  it("cambios recientes, sin barra", () => {
    const s = { ...base, running: true, phase: "updating" as const };
    expect(statusLine(s, 10, 1)).toEqual({
      text: "Actualizando lo que acabas de cambiar…",
      progress: null,
    });
  });

  it("avisa si no hay vigilancia o lector de PDF", () => {
    expect(statusNotes(base)).toEqual([]);
    const notes = statusNotes({ ...base, watching: false, readsPdf: false });
    expect(notes).toHaveLength(2);
    expect(notes[1]).toContain("PDF");
  });

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
