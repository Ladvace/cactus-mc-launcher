import type { Instance } from "$lib/types";

class UiStore {
  createInstanceOpen = $state(false);
  accountsOpen = $state(false);
  commandPaletteOpen = $state(false);
  changelogOpen = $state(false);
  craftingOpen = $state(false);

  instanceMenu = $state<{ instance: Instance; x: number; y: number } | null>(
    null
  );
  groupMenu = $state<{ name: string; x: number; y: number } | null>(null);
  stickerPicker = $state<{
    title: string;
    onPick: (dataUri: string) => void;
  } | null>(null);

  groupFor = $state<Instance | null>(null);
  folderOpen = $state<string | null>(null);
  /// Instance to pre-select as the install target when opening Browse from an
  /// instance's "Find mods" button (so installs land on the right instance).
  browseInstanceId = $state<string | null>(null);

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
  toggleCommandPalette() {
    this.commandPaletteOpen = !this.commandPaletteOpen;
  }
  closeCommandPalette() {
    this.commandPaletteOpen = false;
  }
  openChangelog() {
    this.changelogOpen = true;
  }
  closeChangelog() {
    this.changelogOpen = false;
  }
  openCrafting() {
    this.craftingOpen = true;
  }
  closeCrafting() {
    this.craftingOpen = false;
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

  openFolder(name: string) {
    this.folderOpen = name;
  }
  closeFolder() {
    this.folderOpen = null;
  }
}

export const ui = new UiStore();
