// Global toasts: integration errors from `app://error`, failed commands and short info messages.
import { toAppError } from "$lib/api";
import { errorText } from "$lib/strings";

export type ToastKind = "error" | "info";

export interface Toast {
  id: number;
  kind: ToastKind;
  text: string;
  /** Technical detail (English, from the shell) shown on hover only. */
  detail: string | null;
}

const MAX_TOASTS = 4;
const TTL_INFO_MS = 5000;
const TTL_ERROR_MS = 8000;

class ToastStore {
  items = $state<Toast[]>([]);
  private nextId = 1;
  private timers = new Map<number, ReturnType<typeof setTimeout>>();

  push(text: string, kind: ToastKind = "info", detail: string | null = null): void {
    // Same text already visible → just restart its timer instead of stacking duplicates.
    const existing = this.items.find((t) => t.text === text && t.kind === kind);
    if (existing) {
      this.arm(existing.id, kind);
      return;
    }
    const toast: Toast = { id: this.nextId++, kind, text, detail };
    this.items = [...this.items, toast].slice(-MAX_TOASTS);
    for (const id of this.timers.keys()) {
      if (!this.items.some((t) => t.id === id)) this.clearTimer(id);
    }
    this.arm(toast.id, kind);
  }

  /** Shows any thrown value / `AppError` as a Polish error toast. */
  error(e: unknown): void {
    const err = toAppError(e);
    console.warn(`[FocusMode] ${err.code}: ${err.message}`);
    this.push(errorText(err), "error", err.message || null);
  }

  dismiss(id: number): void {
    this.clearTimer(id);
    this.items = this.items.filter((t) => t.id !== id);
  }

  private arm(id: number, kind: ToastKind): void {
    this.clearTimer(id);
    this.timers.set(
      id,
      setTimeout(() => this.dismiss(id), kind === "error" ? TTL_ERROR_MS : TTL_INFO_MS),
    );
  }

  private clearTimer(id: number): void {
    const t = this.timers.get(id);
    if (t !== undefined) clearTimeout(t);
    this.timers.delete(id);
  }
}

export const toasts = new ToastStore();
