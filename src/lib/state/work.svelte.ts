// "Co dalej?": ranked work items from the connected developer tools (GitHub, GitLab, Jira, Azure DevOps, Linear),
// the state of every tool, the token commands and the session goal. The status is pushed by the shell
// (`work://changed`) and revision-guarded here too, because the commands below also return a status.
// Access tokens are secrets: they live only in the Settings input until they are sent, are never kept in this
// store, and never appear in errors, toasts or logs. Nothing here sends work items anywhere.
import * as api from "$lib/api";
import { errorText, strings } from "$lib/strings";
import type { AppError, Config, SessionGoal, WorkItem, WorkSource, WorkSourceStatus, WorkStatus } from "$lib/types";
import { config } from "./config.svelte";
import { redactLinks } from "./calendar.svelte";
import { session } from "./session.svelte";
import { toasts } from "./toasts.svelte";

/** Every tool, in the order of the Settings section (keys of `strings.work.sourceName`). */
export const WORK_SOURCES = Object.keys(strings.work.sourceName) as WorkSource[];

/** The config switch of each tool (`config.work[WORK_SWITCH[source]]`). */
export const WORK_SWITCH = {
  github: "github",
  gitlab: "gitlab",
  jira: "jira",
  azureDevops: "azureDevops",
  linear: "linear",
} as const satisfies Record<WorkSource, keyof Config["work"]>;

/** Long runs of token-like characters (40+), cut out of shell / server details in case one echoes a credential. */
const TOKEN_LIKE = /[A-Za-z0-9_\-+/=.]{40,}/g;

/** Removes links and anything that looks like a token from a (shell / server) detail before it is shown. */
export function redactDetail(text: string): string {
  return redactLinks(text).replace(TOKEN_LIKE, "[…]").trim();
}

/** Shows a work-tools error as a Polish toast. Unlike `toasts.error` it never logs the detail and redacts it. */
export function reportWorkError(e: unknown): void {
  const err: AppError = api.toAppError(e);
  if (err.code === api.ERR_NO_SHELL) return;
  toasts.push(errorText(err), "error", redactDetail(err.message) || null);
}

/** The tool is switched on in the config. */
export function sourceEnabled(cfg: Config | null, source: WorkSource): boolean {
  const work = cfg?.work as Config["work"] | undefined;
  return work !== undefined && work[WORK_SWITCH[source]] === true;
}

/** At least one tool is switched on in the config. */
export function workEnabled(cfg: Config | null): boolean {
  return WORK_SOURCES.some((source) => sourceEnabled(cfg, source));
}

/** `unavailable` details of the shell (`work/model.rs` `missing`): the address is checked before the token. */
const NO_TOKEN = /\badd an access token\b/i;
const NO_SECRET_STORE = /\bencrypted token storage\b/i;
/** `on` detail before the first successful fetch. */
const NOT_SYNCED = /\bnot synced yet\b/i;

/**
 * A token may be stored: the tool is set up (connected, or failed with a token), names an account, or still lacks
 * its address (the shell checks the address first, so the token is unknown then and "Usuń token" stays offered).
 */
export function hasToken(status: WorkSourceStatus | null): boolean {
  if (status === null || status.state === "off") return false;
  if (status.state === "unavailable") {
    const detail = status.detail ?? "";
    return status.account !== null || !(NO_TOKEN.test(detail) || NO_SECRET_STORE.test(detail));
  }
  return true;
}

/** This system cannot store tokens encrypted (the shell never stores them in plain text). */
export function noSecretStore(status: WorkSourceStatus | null): boolean {
  return status?.state === "unavailable" && NO_SECRET_STORE.test(status.detail ?? "");
}

/** Connected, but the first fetch has not finished yet. */
export function notSyncedYet(status: WorkSourceStatus | null): boolean {
  return status?.state === "on" && NOT_SYNCED.test(status.detail ?? "");
}

/** Mirrors the shell's `validate_token`: after trimming, 1..=4096 bytes of visible ASCII (it goes into an HTTP header). */
export const MAX_TOKEN_BYTES = 4096;
export function isValidToken(raw: string): boolean {
  const token = raw.trim();
  return token.length > 0 && token.length <= MAX_TOKEN_BYTES && /^[\x21-\x7e]+$/.test(token);
}

/** Only http(s) links are opened (focus-core already empties others; checked again before the opener). */
export function isOpenableUrl(url: string): boolean {
  return /^https?:\/\/\S+$/i.test(url.trim());
}

/** The goal for an item (what the shell keeps in the snapshot and the summary). */
export function goalOf(item: WorkItem): SessionGoal {
  return { source: item.source, key: item.key, title: item.title, url: item.url };
}

/** The goal names this item (same tool and key; title and link may have changed since). */
export function isGoalItem(goal: SessionGoal | null, item: WorkItem): boolean {
  return goal !== null && goal.source === item.source && goal.key === item.key && (goal.key !== "" || goal.title === item.title);
}

class WorkStore {
  status = $state.raw<WorkStatus | null>(null);
  refreshing = $state(false);
  /** A `set_work_token` call is in flight for these tools (the buttons wait). */
  tokenBusy = $state<Partial<Record<WorkSource, boolean>>>({});
  /** A `set_session_goal` call is in flight. */
  goalBusy = $state(false);

  apply(status: WorkStatus): void {
    const current = this.status;
    if (current && !api.isNewerRevision(status.revision, current.revision)) return;
    this.status = status;
  }

  source(source: WorkSource): WorkSourceStatus | null {
    return this.status?.sources.find((s) => s.source === source) ?? null;
  }

  /**
   * Reads the current status: the user is looking at the list now, so the shell counts its items as seen (from the
   * next status on, which clears "nowe" and the held count). Failures stay quiet.
   */
  async reload(): Promise<void> {
    try {
      this.apply(await api.getWork());
    } catch {
      // The next `work://changed` brings it anyway.
    }
  }

  /** Fetches all tools now (pending config edits are saved first, so the shell reads the current addresses). */
  async refresh(): Promise<void> {
    if (this.refreshing) return;
    this.refreshing = true;
    try {
      if (!(await config.flush())) return;
      this.apply(await api.refreshWork());
    } catch (e) {
      reportWorkError(e);
    } finally {
      this.refreshing = false;
    }
  }

  /**
   * Stores the token of a tool (`token` trimmed; the caller clears its input on success) or forgets it (`null`).
   * Pending config edits (address, e-mail, organisation) are saved first, so the shell checks the token against them.
   * Resolves true when the shell accepted it.
   */
  async setToken(source: WorkSource, token: string | null): Promise<boolean> {
    if (this.tokenBusy[source]) return false;
    const value = token === null ? null : token.trim();
    if (value !== null && !isValidToken(value)) return false;
    this.tokenBusy[source] = true;
    try {
      if (!(await config.flush())) return false;
      this.apply(await api.setWorkToken(source, value));
      return true;
    } catch (e) {
      reportWorkError(e);
      return false;
    } finally {
      delete this.tokenBusy[source];
    }
  }

  /** Sets (or with `null` clears) the session goal; works in every phase. Resolves true when the shell took it. */
  async setGoal(goal: SessionGoal | null): Promise<boolean> {
    if (this.goalBusy) return false;
    this.goalBusy = true;
    try {
      await api.setSessionGoal(goal);
      // The shell emits a snapshot with the goal; read it too, so the goal shows at once also while idle.
      await session.reload();
      return true;
    } catch (e) {
      reportWorkError(e);
      return false;
    } finally {
      this.goalBusy = false;
    }
  }

  /** Opens an item's (or the goal's) link in the default browser. */
  async open(url: string): Promise<void> {
    if (!isOpenableUrl(url)) return;
    try {
      await api.openExternal(url.trim());
    } catch (e) {
      reportWorkError(e);
    }
  }
}

export const work = new WorkStore();
