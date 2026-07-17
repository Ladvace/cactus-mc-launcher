import { describe, it, expect, beforeEach } from "vitest";
import { render, screen, fireEvent } from "@testing-library/svelte";
import InstancePicker from "./InstancePicker.svelte";
import { instancesStore } from "$lib/stores/instances.svelte";
import type { Instance } from "$lib/types";

function inst(id: string, name: string, group: string | null = null): Instance {
  return {
    id,
    name,
    kind: "client",
    icon: null,
    mcVersion: "1.20.1",
    loader: "fabric",
    loaderVersion: null,
    group,
    created: "",
    lastPlayed: null,
    totalPlaytimeSeconds: 0,
    coverImage: false,
    serverMemoryMb: null,
    maxMemoryMb: null,
    minMemoryMb: null,
    jvmArgs: null,
    javaPath: null,
    gameWidth: null,
    gameHeight: null,
  };
}

describe("InstancePicker", () => {
  beforeEach(() => {
    instancesStore.instances = [
      inst("1", "Alpha"),
      inst("2", "Beta", "Modpacks"),
      inst("3", "Gamma"),
    ];
  });

  it("opens and lists every instance, grouped by folder", async () => {
    render(InstancePicker, { props: { value: "" } });
    await fireEvent.click(screen.getByText("Select an instance"));
    expect(screen.getByText("Alpha")).toBeInTheDocument();
    expect(screen.getByText("Gamma")).toBeInTheDocument();
    expect(screen.getByText("Beta")).toBeInTheDocument();
    // the folder section header for the grouped instance
    expect(screen.getByText("Modpacks")).toBeInTheDocument();
  });

  it("filters by the search query", async () => {
    render(InstancePicker, { props: { value: "" } });
    await fireEvent.click(screen.getByText("Select an instance"));
    const search = screen.getByPlaceholderText("Search instances…");
    await fireEvent.input(search, { target: { value: "beta" } });
    expect(screen.getByText("Beta")).toBeInTheDocument();
    expect(screen.queryByText("Alpha")).toBeNull();
    expect(screen.queryByText("Gamma")).toBeNull();
  });
});
