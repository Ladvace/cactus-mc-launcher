import { describe, it, expect } from "vitest";
import { followedBoards } from "./followedBoards.svelte";

describe("followedBoards", () => {
  it("follows and reports following", () => {
    followedBoards.follow("cactusking");
    expect(followedBoards.isFollowing("cactusking")).toBe(true);
  });

  it("does not duplicate a follow", () => {
    followedBoards.follow("dup");
    followedBoards.follow("dup");
    expect(followedBoards.handles.filter((h) => h === "dup")).toHaveLength(1);
  });

  it("unfollows", () => {
    followedBoards.follow("temp");
    followedBoards.unfollow("temp");
    expect(followedBoards.isFollowing("temp")).toBe(false);
  });

  it("persists to localStorage", () => {
    followedBoards.follow("persist");
    const raw = JSON.parse(localStorage.getItem("cactus:followedBoards") || "[]");
    expect(raw).toContain("persist");
  });
});
