import { describe, expect, it } from "vitest";
import type { IndexStatus, Status } from "../bindings";
import { hotkeyLabel, statusCards } from "./statusCards";

const status: Status = {
  acrylic: true,
  hotkey: "Alt+Space",
  backend: "registerHotKey",
  latency: null,
  storageError: null,
  searchLatency: null,
};

const smart = {
  enabled: true,
  ready: true,
  done: 10,
  total: 10,
  downloadedMb: 0,
  downloadMb: 0,
  awaitingPermission: false,
  note: null as string | null,
};

const index = (over: Partial<IndexStatus> = {}): IndexStatus => ({
  running: false,
  phase: "idle",
  currentRoot: null,
  scanned: 0,
  done: 0,
  total: 0,
  last: null,
  error: null,
  watching: true,
  readsPdf: true,
  smart,
  ...over,
});

const roots = { folders: 2, files: 1240 };
const byId = (cards: ReturnType<typeof statusCards>) =>
  Object.fromEntries(cards.map((c) => [c.id, c]));

describe("statusCards", () => {
  it("todo bien: cuatro tarjetas en verde, en lenguaje normal", () => {
    const cards = statusCards(status, index(), roots);
    expect(cards.map((c) => [c.title, c.tone])).toEqual([
      ["Atajo funcionando", "ok"],
      ["Carpetas al día", "ok"],
      ["Búsqueda inteligente lista", "ok"],
      ["Tus datos se guardan bien", "ok"],
    ]);
    expect(cards[0].detail).toContain("Alt+Espacio");
    expect(cards[1].detail).toBe("1.240 archivos listos para buscar.");
  });

  it("sin atajo o sin poder guardar: en rojo y diciendo qué hacer", () => {
    const c = byId(
      statusCards(
        { ...status, hotkey: null, storageError: "disco lleno" },
        index(),
        roots,
      ),
    );
    expect(c.hotkey).toMatchObject({
      tone: "error",
      title: "El atajo no funciona",
    });
    expect(c.hotkey.detail).toContain("General");
    expect(c.data).toMatchObject({ tone: "error", detail: "disco lleno" });
  });

  it("leyendo carpetas: en marcha, con el progreso", () => {
    const c = byId(
      statusCards(
        status,
        index({ running: true, phase: "reading", done: 31, total: 120 }),
        roots,
      ),
    );
    expect(c.folders).toMatchObject({
      tone: "working",
      title: "Leyendo tus carpetas",
      detail: "Leyendo 32 de 120 archivos",
    });
  });

  it("sin carpetas: aviso para añadir una", () => {
    const c = byId(statusCards(status, index(), { folders: 0, files: 0 }));
    expect(c.folders).toMatchObject({ tone: "warning", title: "Sin carpetas" });
  });

  it("búsqueda inteligente: apagada, sin permiso, con problemas o preparándose", () => {
    const smartCard = (s: Partial<typeof smart>) =>
      byId(statusCards(status, index({ smart: { ...smart, ...s } }), roots))
        .smart;
    expect(smartCard({ enabled: false }).tone).toBe("off");
    expect(smartCard({ awaitingPermission: true, ready: false }).tone).toBe(
      "warning",
    );
    expect(smartCard({ note: "sin espacio" })).toMatchObject({
      tone: "warning",
      detail: "sin espacio",
    });
    expect(smartCard({ ready: false, done: 3, total: 10 })).toMatchObject({
      tone: "working",
      title: "Preparando la búsqueda inteligente",
    });
  });

  it("el atajo se dice como en un teclado español", () => {
    expect(hotkeyLabel("Ctrl+Alt+Space")).toBe("Ctrl+Alt+Espacio");
  });
});
