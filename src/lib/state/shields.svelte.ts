// Shield status (notifications / Teams / sites / keep awake) pushed by the shell.
import * as api from "$lib/api";
import type { BridgeInfo, ShieldStatus } from "$lib/types";
import { toasts } from "./toasts.svelte";

class ShieldsStore {
  status = $state<ShieldStatus | null>(null);
  bridge = $state<BridgeInfo | null>(null);

  apply(status: ShieldStatus): void {
    this.status = status;
  }

  async refreshBridge(): Promise<void> {
    try {
      this.bridge = await api.getBridgeInfo();
    } catch (e) {
      const err = api.toAppError(e);
      if (err.code !== api.ERR_NO_SHELL) toasts.error(err);
    }
  }

  async openExtensionDir(): Promise<void> {
    try {
      await api.openExtensionDir();
    } catch (e) {
      toasts.error(e);
    }
  }
}

export const shields = new ShieldsStore();
