export type ToastKind = "success" | "error" | "info";

export interface Toast {
  id: number;
  kind: ToastKind;
  message: string;
}

/// Global, app-wide toast notifications. Errors linger longer and are copiable
/// (see Toaster.svelte).
class ToastStore {
  toasts = $state<Toast[]>([]);
  private nextId = 1;

  private push(kind: ToastKind, message: string, ttl: number): number {
    const id = this.nextId++;
    this.toasts = [...this.toasts, { id, kind, message }];
    if (ttl > 0) setTimeout(() => this.dismiss(id), ttl);
    return id;
  }

  success(message: string) {
    return this.push("success", message, 3500);
  }
  info(message: string) {
    return this.push("info", message, 4500);
  }
  /** Errors stay ~12s (and can be dismissed) so they can be read + copied. */
  error(message: string) {
    return this.push("error", message, 12000);
  }

  dismiss(id: number) {
    this.toasts = this.toasts.filter((existing) => existing.id !== id);
  }
}

export const toast = new ToastStore();
