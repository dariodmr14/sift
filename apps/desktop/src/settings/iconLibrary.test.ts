import { describe, expect, it } from "vitest";
import { buildSvg, humanize, searchIcons, type IconNode } from "./iconLibrary";

describe("buildSvg", () => {
  it("genera un SVG con trazo currentColor", () => {
    const svg = buildSvg([["circle", { cx: 12, cy: 12, r: "10" }]]);
    expect(svg.startsWith("<svg")).toBe(true);
    expect(svg).toContain('stroke="currentColor"');
    expect(svg).toContain('<circle cx="12" cy="12" r="10"/>');
  });

  it("no deja colar etiquetas, eventos ni comillas", () => {
    const evil: IconNode = [
      ["path", { d: 'M0 0"/><script>alert(1)</script>', onload: "alert(1)" }],
      ["script>alert", {}],
    ];
    const svg = buildSvg(evil);
    expect(svg).not.toContain("<script");
    expect(svg).not.toContain("onload");
    expect(svg).toContain("&quot;");
  });
});

describe("humanize", () => {
  it("separa CamelCase en palabras", () => {
    expect(humanize("ArrowDownCircle")).toBe("arrow down circle");
    expect(humanize("Gamepad2")).toBe("gamepad2");
  });
});

describe("searchIcons", () => {
  const names = [
    "Database",
    "DatabaseBackup",
    "Folder",
    "FolderOpen",
    "Globe",
    "BookOpen",
    "Notebook",
  ];
  const found = (q: string) => searchIcons(q, names).map((h) => h.name);

  it("busca en inglés", () => {
    expect(found("folder")).toEqual(["Folder", "FolderOpen"]);
  });

  it("entiende palabras en español", () => {
    expect(found("carpeta")).toEqual(["Folder", "FolderOpen"]);
    expect(found("base de datos")).toEqual(["Database", "DatabaseBackup"]); // "de" se ignora
    expect(found("base")).toEqual(["Database", "DatabaseBackup"]);
    expect(found("apuntes")).toEqual(["BookOpen", "Notebook"]);
  });

  it("sin búsqueda devuelve los primeros", () => {
    expect(searchIcons("", names, 3)).toHaveLength(3);
  });
});
