import { describe, it, expect } from "vitest";
import { groupCovers } from "./groupCovers.svelte";

describe("groupCovers", () => {
  it("stores and reads a cover", () => {
    groupCovers.set("Modpacks", "data:uri-1");
    expect(groupCovers.get("Modpacks")).toBe("data:uri-1");
  });

  it("returns null for an unknown group", () => {
    expect(groupCovers.get("nope")).toBeNull();
  });

  it("clears a cover", () => {
    groupCovers.set("Temp", "x");
    groupCovers.clear("Temp");
    expect(groupCovers.get("Temp")).toBeNull();
  });

  it("moves the cover when a folder is renamed", () => {
    groupCovers.set("Old", "data:uri-2");
    groupCovers.rename("Old", "New");
    expect(groupCovers.get("Old")).toBeNull();
    expect(groupCovers.get("New")).toBe("data:uri-2");
  });

  it("persists to localStorage", () => {
    groupCovers.set("Persisted", "data:uri-3");
    const raw = JSON.parse(localStorage.getItem("cactus:groupCovers") || "{}");
    expect(raw.Persisted).toBe("data:uri-3");
  });
});
