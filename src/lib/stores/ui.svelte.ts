import type { Instance } from "$lib/types";

/// Small shared UI state (modals, transient flags).
class UiStore {
  createInstanceOpen = $state(false);
  accountsOpen = $state(false);

  // Right-click menu on an instance tile.
  instanceMenu = $state<{ instance: Instance; x: number; y: number } | null>(
    null
  );
  // Sticker/emoji picker: a title + a callback that receives the chosen image
  // as a data URI (null = closed). Used for instance icons and app backgrounds.
  stickerPicker = $state<{
    title: string;
    onPick: (dataUri: string) => void;
  } | null>(null);

  // "Move to group" picker target instance (null = closed).
  groupFor = $state<Instance | null>(null);

  openCreateInstance() {
    this.createInstanceOpen = true;
  }
  closeCreateInstance() {
    this.createInstanceOpen = false;
  }

  openAccounts() {
    this.accountsOpen = true;
  }
  closeAccounts() {
    this.accountsOpen = false;
  }

  openInstanceMenu(instance: Instance, x: number, y: number) {
    this.instanceMenu = { instance, x, y };
  }
  closeInstanceMenu() {
    this.instanceMenu = null;
  }

  openStickerPicker(title: string, onPick: (dataUri: string) => void) {
    this.instanceMenu = null;
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
