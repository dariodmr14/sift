import { describe, expect, it } from "vitest";
import { since } from "./FoldersSection";

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
