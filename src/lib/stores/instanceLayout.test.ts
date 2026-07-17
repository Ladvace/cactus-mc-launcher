import { describe, it, expect } from "vitest";
import { instanceLayout } from "./instanceLayout.svelte";

describe("instanceLayout", () => {
  it("returns a default 1x1 cell (last order) for unknown ids", () => {
    const c = instanceLayout.cellOf("unknown");
    expect(c.w).toBe(1);
    expect(c.h).toBe(1);
    expect(c.order).toBe(Number.MAX_SAFE_INTEGER);
  });

  it("stores a set cell", () => {
    instanceLayout.set("a", { w: 2, h: 1, order: 3 });
    expect(instanceLayout.cellOf("a")).toEqual({ w: 2, h: 1, order: 3 });
  });

  it("reorders ids while preserving each span", () => {
    instanceLayout.set("x", { w: 2, h: 2, order: 5 });
    instanceLayout.set("y", { w: 1, h: 1, order: 9 });
    instanceLayout.reorder(["y", "x"]);
    expect(instanceLayout.cellOf("y").order).toBe(0);
    expect(instanceLayout.cellOf("x").order).toBe(1);
    expect(instanceLayout.cellOf("x").w).toBe(2);
    expect(instanceLayout.cellOf("x").h).toBe(2);
  });
});
