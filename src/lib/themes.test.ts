import { describe, it, expect } from "vitest";
import { THEME_PRESETS, DECOR_THEMES } from "./themes";

const SPRITE = /^\/decor\/sprites\/\d+\.png$/;

describe("theme presets", () => {
  it("include Default and have unique names", () => {
    const names = THEME_PRESETS.map((t) => t.name);
    expect(names).toContain("Default");
    expect(new Set(names).size).toBe(names.length);
  });

  it("every decor preset references a defined decor theme", () => {
    for (const t of THEME_PRESETS) {
      if (t.decor) expect(DECOR_THEMES.some((d) => d.id === t.decor)).toBe(true);
    }
  });

  it("each decor theme has placements + a peek sprite", () => {
    for (const d of DECOR_THEMES) {
      expect(d.placements.length).toBeGreaterThan(0);
      expect(d.peek).toMatch(SPRITE);
      for (const p of d.placements) {
        expect(p.sprite).toMatch(SPRITE);
        expect(p.size).toBeGreaterThan(0);
      }
    }
  });
});
