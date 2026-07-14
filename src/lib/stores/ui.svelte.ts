/// Small shared UI state (modals, transient flags).
class UiStore {
  createInstanceOpen = $state(false);
  accountsOpen = $state(false);

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
}

export const ui = new UiStore();
