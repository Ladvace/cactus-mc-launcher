export type ToastKind = "success" | "error" | "info";

export interface ToastAction {
  label: string;
  run: () => void;
}

export interface Toast {
  id: number;
  kind: ToastKind;
  message: string;
  action?: ToastAction;
}

type Timer = { handle: ReturnType<typeof setTimeout>; remaining: number; startedAt: number };

class ToastStore {
  toasts = $state<Toast[]>([]);
  private nextId = 1;
  private timers = new Map<number, Timer>();
  private paused = false;

  private push(kind: ToastKind, message: string, ttl: number, action?: ToastAction): number {
    const id = this.nextId++;
    this.toasts = [...this.toasts, { id, kind, message, action }];
    if (ttl > 0) this.arm(id, ttl);
    return id;
  }

  private arm(id: number, ms: number) {
    this.timers.set(id, {
      handle: setTimeout(() => this.dismiss(id), ms),
      remaining: ms,
      startedAt: Date.now(),
    });
  }

  success(message: string, action?: ToastAction) {
    return this.push("success", message, action ? 8000 : 3500, action);
  }
  info(message: string, action?: ToastAction) {
    return this.push("info", message, action ? 8000 : 4500, action);
  }
  error(message: string, action?: ToastAction) {
    return this.push("error", message, 12000, action);
  }

  run(id: number, action: ToastAction) {
    this.dismiss(id);
    action.run();
  }

  // Freeze the auto-dismiss countdowns while the stack is hovered/expanded.
  pause() {
    if (this.paused) return;
    this.paused = true;
    for (const timer of this.timers.values()) {
      clearTimeout(timer.handle);
      timer.remaining -= Date.now() - timer.startedAt;
    }
  }

  resume() {
    if (!this.paused) return;
    this.paused = false;
    for (const [id, timer] of this.timers) {
      this.arm(id, Math.max(timer.remaining, 600));
    }
  }

  dismiss(id: number) {
    const timer = this.timers.get(id);
    if (timer) clearTimeout(timer.handle);
    this.timers.delete(id);
    this.toasts = this.toasts.filter((existing) => existing.id !== id);
  }
}

export const toast = new ToastStore();
