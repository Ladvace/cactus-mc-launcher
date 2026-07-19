import type { Instance } from "$lib/types";

/// Small shared UI state (modals, transient flags).
class UiStore {
  createInstanceOpen = $state(false);
  // Optional prefill for the create modal: preselect a version and, after
  // creating, auto-join a server (used by the Servers page quick-connect).
  createPrefill = $state<{ mcVersion?: string; joinServer?: string } | null>(null);
  accountsOpen = $state(false);

  // Right-click menu on an instance tile.
  instanceMenu = $state<{ instance: Instance; x: number; y: number } | null>(
    null
  );
  // Right-click menu on a group (folder) tile.
  groupMenu = $state<{ name: string; x: number; y: number } | null>(null);
  // Sticker/emoji picker: a title + a callback that receives the chosen image
  // as a data URI (null = closed). Used for instance icons and app backgrounds.
  stickerPicker = $state<{
    title: string;
    onPick: (dataUri: string) => void;
  } | null>(null);

  // "Move to group" picker target instance (null = closed).
  groupFor = $state<Instance | null>(null);

  openCreateInstance(prefill?: { mcVersion?: string; joinServer?: string }) {
    this.createPrefill = prefill ?? null;
    this.createInstanceOpen = true;
  }
  closeCreateInstance() {
    this.createInstanceOpen = false;
    this.createPrefill = null;
  }

  openAccounts() {
    this.accountsOpen = true;
  }
  closeAccounts() {
    this.accountsOpen = false;
  }

  openInstanceMenu(instance: Instance, x: number, y: number) {
    this.groupMenu = null;
    this.instanceMenu = { instance, x, y };
  }
  closeInstanceMenu() {
    this.instanceMenu = null;
  }

  openGroupMenu(name: string, x: number, y: number) {
    this.instanceMenu = null;
    this.groupMenu = { name, x, y };
  }
  closeGroupMenu() {
    this.groupMenu = null;
  }

  openStickerPicker(title: string, onPick: (dataUri: string) => void) {
    this.instanceMenu = null;
    this.groupMenu = null;
    this.stickerPicker = { title, onPick };
  }
  closeStickerPicker() {
    this.stickerPicker = null;
  }

  openGroupPicker(instance: Instance) {
    this.instanceMenu = null;
    this.groupFor = instance;
  }
  closeGroupPicker() {
    this.groupFor = null;
  }
}

export const ui = new UiStore();
