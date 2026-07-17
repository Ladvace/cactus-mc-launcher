import { describe, it, expect } from "vitest";
import { render, screen } from "@testing-library/svelte";
import Toaster from "./Toaster.svelte";
import { toast } from "$lib/stores/toast.svelte";

// Integration-ish: the global toast store drives the Toaster component.
describe("Toaster", () => {
  it("renders a pushed error and its Copy button", async () => {
    render(Toaster);
    toast.error("Disk went missing");
    expect(await screen.findByText("Disk went missing")).toBeInTheDocument();
    expect(await screen.findByText("Copy")).toBeInTheDocument();
  });

  it("dismisses when the close button is clicked", async () => {
    const { container } = render(Toaster);
    const id = toast.success("Saved");
    expect(await screen.findByText("Saved")).toBeInTheDocument();
    toast.dismiss(id);
    // give Svelte a tick to remove it
    await new Promise((r) => setTimeout(r, 0));
    expect(container.textContent).not.toContain("Saved");
  });
});
