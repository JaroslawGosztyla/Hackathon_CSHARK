// Site blocklist: the shell parses `websiteblacklist.txt` (the only source of the list, ADR-013) and pushes
// the result as `blocklist://changed`; the file itself is edited in the system's text editor.
import * as api from "$lib/api";
import { strings } from "$lib/strings";
import type { BlocklistInfo } from "$lib/types";
import { toasts } from "./toasts.svelte";

class BlocklistStore {
  info = $state<BlocklistInfo | null>(null);
  opening = $state(false);
  reloading = $state(false);

  /** Active entries (domains + regexes); null before the first read. */
  count = $derived(this.info ? this.info.domains.length + this.info.patterns.length : null);

  /**
   * Takes a pushed, fetched or reloaded info. `subscribe` already filters events against the getter, but a
   * `reload_blocklist` result races with them, so only a strictly newer revision replaces what is shown (ADR-012).
   */
  apply(info: BlocklistInfo): void {
    const current = this.info;
    if (current && !api.isNewerRevision(info.revision, current.revision)) return;
    this.info = info;
  }

  async open(): Promise<void> {
    if (this.opening) return;
    this.opening = true;
    try {
      await api.openBlocklistFile();
    } catch (e) {
      toasts.error(e);
    } finally {
      this.opening = false;
    }
  }

  async reload(): Promise<void> {
    if (this.reloading) return;
    this.reloading = true;
    try {
      const info = await api.reloadBlocklist();
      this.apply(info);
      if (info.error) toasts.error({ code: "blocklist.read", message: info.error, field: null });
      else toasts.push(strings.blocklist.reloaded);
    } catch (e) {
      toasts.error(e);
    } finally {
      this.reloading = false;
    }
  }
}

export const blocklist = new BlocklistStore();
