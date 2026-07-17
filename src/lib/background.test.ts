import { describe, it, expect } from "vitest";
import {
  backgroundCss,
  bgKind,
  parsePattern,
  parseImage,
  parseTile,
} from "./background";

describe("bgKind", () => {
  it("classifies each stored form", () => {
    expect(bgKind("")).toBe("default");
    expect(bgKind("color:#123456")).toBe("color");
    expect(bgKind("pattern:dots")).toBe("pattern");
    expect(bgKind("image:data:x")).toBe("image");
    expect(bgKind("tile:/a.png")).toBe("tile");
  });
});

describe("parsePattern", () => {
  it("splits name and optional colour", () => {
    expect(parsePattern("pattern:dots")).toEqual({ name: "dots", color: null });
    expect(parsePattern("pattern:grid|#101010")).toEqual({
      name: "grid",
      color: "#101010",
    });
  });
});

describe("parseImage / parseTile", () => {
  it("keeps a data URI intact (no leading #)", () => {
    const uri = "data:image/png;base64,AAAA";
    expect(parseImage(`image:${uri}`)).toEqual({ uri, color: null });
  });
  it("extracts a leading colour before the pipe", () => {
    expect(parseImage("image:#abcdef|http://x/y.png")).toEqual({
      color: "#abcdef",
      uri: "http://x/y.png",
    });
    expect(parseTile("tile:#001122|/decor/1.png")).toEqual({
      color: "#001122",
      uri: "/decor/1.png",
    });
  });
});

describe("backgroundCss", () => {
  it("falls back to the app colour for default", () => {
    expect(backgroundCss("")).toBe("var(--bg-app)");
  });
  it("returns a solid colour verbatim", () => {
    expect(backgroundCss("color:#ff0000")).toBe("#ff0000");
  });
  it("passes a gradient through the colour kind", () => {
    const g = "linear-gradient(160deg, #d98a2b 0%, #3a2340 100%)";
    expect(backgroundCss(`color:${g}`)).toBe(g);
  });
  it("uses the chosen base colour in a pattern", () => {
    expect(backgroundCss("pattern:diagonal|#123456")).toContain("#123456");
  });
  it("builds a scrim + cover for an image with a tint", () => {
    const css = backgroundCss("image:#000000|/x.png");
    expect(css).toContain('url("/x.png")');
    expect(css).toContain("rgba(0, 0, 0,");
    expect(css).toContain("cover");
  });
  it("repeats a decor sprite for a tile", () => {
    const css = backgroundCss("tile:#111111|/decor/56.png");
    expect(css).toContain('url("/decor/56.png")');
    expect(css).toContain("repeat");
    expect(css).toContain("#111111");
  });
});
