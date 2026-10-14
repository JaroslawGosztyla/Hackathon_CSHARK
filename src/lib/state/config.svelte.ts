// Config store: loaded once, edited in place, saved (debounced) through `update_config`.
// The shell normalizes + validates; its validation errors land in `fieldErrors` next to the field.
import * as api from "$lib/api";
import { errorText, strings } from "$lib/strings";
import type { Config } from "$lib/types";
import { toasts } from "./toasts.svelte";

const SAVE_DEBOUNCE_MS = 600;
const FOCUS_MINUTES = { min: 1, max: 240 } as const;
const BREAK_MINUTES = { min: 1, max: 120 } as const;
/**
 * Mirrors focus-core `tint.rs`: times are minutes after midnight, strength 0..=100, fade 0..=180 min; and `config.rs`
 * for ADR-023 (MAX_PAUSE_APPS, MAX_PROCESS_NAME_CHARS, LATITUDE_RANGE, LONGITUDE_RANGE).
 */
const TINT = {
  minutesPerDay: 1440,
  maxStrength: 100,
  maxTransitionMinutes: 180,
  maxPauseApps: 64,
  maxProcessNameChars: 128,
  maxLatitude: 90,
  maxLongitude: 180,
} as const;
/**
 * Mirrors focus-core `tint::DEFAULT_PAUSE_APPS` (ADR-023): the colour-critical apps of a fresh config, for "Restore
 * the default list". Matched case-insensitively, so `Lightroom.exe` covers Lightroom Classic too.
 */
export const DEFAULT_TINT_PAUSE_APPS: readonly string[] = [
  "Photoshop.exe",
  "Illustrator.exe",
  "Lightroom.exe",
  "AfterFX.exe",
  "Adobe Premiere Pro.exe",
  "InDesign.exe",
  "Figma.exe",
  "Affinity Photo 2.exe",
  "Affinity Designer 2.exe",
  "Affinity Publisher 2.exe",
  "gimp-2.10.exe",
  "gimp-3.0.exe",
  "krita.exe",
  "Resolve.exe",
  "blender.exe",
  "Substance 3D Painter.exe",
  "CaptureOne.exe",
];

/**
 * Mirrors focus-core `config::is_process_name`: a trimmed exe name as the tracker reports it (`Code.exe`), at most
 * `maxProcessNameChars` characters, no path separators, drive colons or control characters.
 */
export function isProcessName(name: string): boolean {
  return (
    name !== "" &&
    name === name.trim() &&
    charCount(name) <= TINT.maxProcessNameChars &&
    !/[\\/:\p{Cc}]/u.test(name)
  );
}

function isDegrees(value: number, max: number): boolean {
  return Number.isFinite(value) && value >= -max && value <= max;
}
/** Choices offered for the tint fade (minutes); a different stored value is shown as an extra option. */
export const TINT_TRANSITION_PRESETS = [0, 15, 30, 60, 90] as const;
/** Mirrors focus-core `config.rs` (MAX_BREAK_REMINDERS, MAX_REMINDER_CHARS; length counted in characters). */
const REMINDERS = { maxCount: 24, maxChars: 160 } as const;
/**
 * Mirrors focus-core `config.rs` (CALENDAR_HORIZON_HOURS_RANGE, GOOGLE_CLIENT_ID_SUFFIX, MAX_GOOGLE_CLIENT_ID_LEN,
 * MAX_GOOGLE_CLIENT_SECRET_LEN).
 */
const CALENDAR = {
  minHorizonHours: 1,
  maxHorizonHours: 24,
  googleClientIdSuffix: ".apps.googleusercontent.com",
  maxGoogleClientIdLen: 255,
  maxGoogleClientSecretLen: 100,
} as const;
/** Choices offered for the calendar look-ahead (hours); a different stored value is shown as an extra option. */
export const CALENDAR_HORIZON_PRESETS = [2, 4, 6, 8, 10, 12, 16, 24] as const;

/**
 * Mirrors focus-core `is_google_client_id`: at most 255 characters of `[A-Za-z0-9._-]`, ending (case-sensitively) in
 * `.apps.googleusercontent.com` after a non-empty prefix that does not start with `.` / `-`, does not end with `.`
 * and has no `..`.
 */
export function isGoogleClientId(id: string): boolean {
  const suffix = CALENDAR.googleClientIdSuffix;
  if (!id.endsWith(suffix)) return false;
  const prefix = id.slice(0, id.length - suffix.length);
  return (
    id.length <= CALENDAR.maxGoogleClientIdLen &&
    prefix.length > 0 &&
    !prefix.startsWith(".") &&
    !prefix.startsWith("-") &&
    !prefix.endsWith(".") &&
    !prefix.includes("..") &&
    /^[A-Za-z0-9._-]+$/.test(id)
  );
}

/** Mirrors focus-core `config.rs` (MAX_WORK_URL_LEN, MAX_HOST_LEN, MAX_JIRA_EMAIL_LEN, MAX_AZURE_ORG_LEN). */
const WORK = { maxUrlBytes: 512, maxHostLen: 253, maxJiraEmailChars: 254, maxAzureOrgBytes: 64 } as const;

const utf8 = new TextEncoder();

/** Mirrors focus-core `is_host_name` on the lowercased host: labels of `[a-z0-9-]`, not empty, no `-` at either end. */
function isHostName(host: string): boolean {
  return (
    host.length <= WORK.maxHostLen &&
    host.split(".").every((label) => label !== "" && !label.startsWith("-") && !label.endsWith("-") && /^[a-z0-9-]+$/.test(label))
  );
}

/**
 * Mirrors focus-core `normalize_work_url` (accepts exactly what the shell stores): empty, or `https://` (any case) +
 * host + optional port 1..=65535 + optional path of `[A-Za-z0-9._~%-]` segments (no `.` / `..` / empty segment,
 * trailing `/` allowed); no `@`, `?` or `#`; at most 512 bytes after trimming.
 */
export function isWorkUrl(raw: string): boolean {
  const url = raw.trim();
  if (url === "") return true;
  if (utf8.encode(url).length > WORK.maxUrlBytes) return false;
  const scheme = "https://";
  if (url.slice(0, scheme.length).toLowerCase() !== scheme) return false;
  const rest = url.slice(scheme.length);
  if (/[@?#]/.test(rest)) return false;
  const slash = rest.indexOf("/");
  const authority = slash === -1 ? rest : rest.slice(0, slash);
  const path = slash === -1 ? "" : rest.slice(slash);
  const colon = authority.indexOf(":");
  // ASCII-only lowercasing, like Rust's `to_ascii_lowercase` (e.g. the Kelvin sign must stay invalid).
  const host = (colon === -1 ? authority : authority.slice(0, colon)).replace(/[A-Z]/g, (ch) => ch.toLowerCase());
  const port = colon === -1 ? null : authority.slice(colon + 1);
  if (!isHostName(host)) return false;
  if (port !== null && (!/^\d+$/.test(port) || Number(port) < 1 || Number(port) > 65535)) return false;
  const trimmedPath = path.replace(/\/+$/, "");
  return trimmedPath
    .split("/")
    .slice(1)
    .every((segment) => segment !== "" && segment !== "." && segment !== ".." && /^[A-Za-z0-9._~%-]+$/.test(segment));
}

/**
 * Mirrors focus-core `normalize_jira_email`: empty, or at most 254 characters without whitespace, control characters
 * or `:`, with a non-empty part before and after the last `@`.
 */
export function isJiraEmail(raw: string): boolean {
  const email = raw.trim();
  if (email === "") return true;
  if (charCount(email) > WORK.maxJiraEmailChars || /[\s\p{Cc}:]/u.test(email)) return false;
  const at = email.lastIndexOf("@");
  return at > 0 && at < email.length - 1;
}

/**
 * Mirrors focus-core `normalize_azure_org`: a pasted `https://dev.azure.com/<org>/…` or `<org>.visualstudio.com` counts
 * as `<org>`; then empty or at most 64 characters of `[A-Za-z0-9._-]`, not only dots.
 */
export function isAzureOrg(raw: string): boolean {
  let org = raw.trim().replace(/^https?:\/\//i, "");
  const devAzure = /^dev\.azure\.com\//i;
  if (devAzure.test(org)) {
    org = org.replace(devAzure, "").split("/")[0] ?? "";
  } else {
    const host = org.split("/")[0] ?? "";
    const suffix = ".visualstudio.com";
    if (host.length >= suffix.length && host.slice(host.length - suffix.length).toLowerCase() === suffix) {
      org = host.slice(0, host.length - suffix.length);
    }
  }
  if (org !== "" && /^\.+$/.test(org)) return false;
  return utf8.encode(org).length <= WORK.maxAzureOrgBytes && /^[A-Za-z0-9._-]*$/.test(org);
}

/** Mirrors focus-core `is_google_client_secret`: 1..=100 characters of `[A-Za-z0-9_-]` (e.g. `GOCSPX-…`). */
export function isGoogleClientSecret(secret: string): boolean {
  return secret.length > 0 && secret.length <= CALENDAR.maxGoogleClientSecretLen && /^[A-Za-z0-9_-]+$/.test(secret);
}

/**
 * The user's reminder for break number `breakNumber` (1-based; the session's `cycle` during that break).
 * Mirrors focus-core `reminder_for_break`: empty entries mean "no reminder"; past the end of the list it repeats
 * from the start only when `repeat` is on.
 */
export function reminderForBreak(reminders: readonly string[], repeat: boolean, breakNumber: number): string | null {
  const n = Math.trunc(breakNumber);
  if (!Number.isFinite(n) || n < 1 || reminders.length === 0) return null;
  const index = n - 1 < reminders.length ? n - 1 : repeat ? (n - 1) % reminders.length : -1;
  const text = index >= 0 ? (reminders[index] ?? "").trim() : "";
  return text.length > 0 ? text : null;
}

/** Length in characters (code points), like Rust's `chars().count()`. */
export function charCount(text: string): number {
  return Array.from(text).length;
}

function inRange(n: number, min: number, max: number): boolean {
  return Number.isInteger(n) && n >= min && n <= max;
}

function isUnit(n: number): boolean {
  return Number.isFinite(n) && n >= 0 && n <= 1;
}

/** Client-side mirror of the ranges in `api.rs`, so obviously invalid values are never sent. */
export function validateConfig(c: Config): Record<string, string> {
  const errors: Record<string, string> = {};
  if (!inRange(c.focusMinutes, FOCUS_MINUTES.min, FOCUS_MINUTES.max)) errors.focusMinutes = strings.timing.rangeFocus;
  if (!inRange(c.breakMinutes, BREAK_MINUTES.min, BREAK_MINUTES.max)) errors.breakMinutes = strings.timing.rangeBreak;
  if (!isUnit(c.music.volume)) errors["music.volume"] = strings.errors.fieldInvalid(strings.fields["music.volume"] ?? "");
  if (!isUnit(c.chimeVolume)) errors.chimeVolume = strings.errors.fieldInvalid(strings.fields.chimeVolume ?? "");
  // Validated even when the tint is off, like in focus-core (the shell would reject the whole config).
  const t = c.tint;
  if (!inRange(t.startMinutes, 0, TINT.minutesPerDay - 1)) errors["tint.startMinutes"] = strings.settings.tintTimeInvalid;
  if (!inRange(t.endMinutes, 0, TINT.minutesPerDay - 1)) errors["tint.endMinutes"] = strings.settings.tintTimeInvalid;
  else if (t.startMinutes === t.endMinutes) errors["tint.endMinutes"] = strings.settings.tintSameTimes;
  if (!inRange(t.strength, 0, TINT.maxStrength)) errors["tint.strength"] = strings.settings.tintStrengthRange;
  if (!inRange(t.transitionMinutes, 0, TINT.maxTransitionMinutes)) {
    errors["tint.transitionMinutes"] = strings.settings.tintTransitionRange;
  }
  // ADR-023. The fields may be missing only when talking to an older shell build; nothing to check then.
  const location = t.location as Config["tint"]["location"] | undefined;
  if (location) {
    if (!isDegrees(location.latitude, TINT.maxLatitude)) {
      errors["tint.location.latitude"] = strings.settings.tintLatitudeRange;
    }
    if (!isDegrees(location.longitude, TINT.maxLongitude)) {
      errors["tint.location.longitude"] = strings.settings.tintLongitudeRange;
    }
  }
  const pauseApps = (t.pauseApps as string[] | undefined) ?? [];
  const names = pauseApps.map((app) => app.trim()).filter((app) => app !== "");
  if (names.some((app) => !isProcessName(app))) {
    errors["tint.pauseApps"] = strings.settings.tintPauseAppInvalid(TINT.maxProcessNameChars);
  } else if (new Set(names.map((app) => app.toLowerCase())).size > TINT.maxPauseApps) {
    errors["tint.pauseApps"] = strings.settings.tintPauseAppsFull(TINT.maxPauseApps);
  }
  const reminders = c.breakReminders;
  if (reminders.length > REMINDERS.maxCount || reminders.some((r) => charCount(r.trim()) > REMINDERS.maxChars)) {
    errors.breakReminders = strings.settings.remindersInvalid(REMINDERS.maxCount, REMINDERS.maxChars);
  }
  // Validated even when the calendars are off, like in focus-core. The shell trims the client id; empty is allowed.
  // `calendar` may be missing only when talking to an older shell build; nothing to check then.
  const cal = c.calendar as Config["calendar"] | undefined;
  if (cal) {
    if (!inRange(cal.horizonHours, CALENDAR.minHorizonHours, CALENDAR.maxHorizonHours)) {
      errors["calendar.horizonHours"] = strings.errors.fieldInvalid(strings.fields["calendar.horizonHours"] ?? "");
    }
    const clientId = cal.googleClientId.trim();
    if (clientId !== "" && !isGoogleClientId(clientId)) {
      errors["calendar.googleClientId"] = strings.calendar.googleClientIdInvalid;
    }
    // Missing in a config from a shell build before the secret: nothing to check then.
    const clientSecret = (cal.googleClientSecret as string | undefined)?.trim() ?? "";
    if (clientSecret !== "" && !isGoogleClientSecret(clientSecret)) {
      errors["calendar.googleClientSecret"] = strings.calendar.googleClientSecretInvalid(
        CALENDAR.maxGoogleClientSecretLen,
      );
    }
  }
  // Validated even when the tools are off, like in focus-core. Missing only with an older shell build.
  const work = c.work as Config["work"] | undefined;
  if (work) {
    if (!isWorkUrl(work.gitlabUrl)) errors["work.gitlabUrl"] = strings.work.urlInvalid;
    if (!isWorkUrl(work.jiraUrl)) errors["work.jiraUrl"] = strings.work.urlInvalid;
    if (!isJiraEmail(work.jiraEmail)) errors["work.jiraEmail"] = strings.work.emailInvalid;
    if (!isAzureOrg(work.azureOrg)) errors["work.azureOrg"] = strings.work.azureOrgInvalid;
  }
  return errors;
}

export const LIMITS = {
  focusMinutes: FOCUS_MINUTES,
  breakMinutes: BREAK_MINUTES,
  tint: TINT,
  reminders: REMINDERS,
  calendar: CALENDAR,
  work: WORK,
} as const;

function copyField<K extends keyof Config>(target: Config, source: Config, key: K): void {
  target[key] = source[key];
}

class ConfigStore {
  value = $state<Config | null>(null);
  fieldErrors = $state<Record<string, string>>({});
  saving = $state(false);
  /** Bumps after every successful save (for a small "Zapisano" hint). */
  savedCount = $state(0);

  private timer: ReturnType<typeof setTimeout> | null = null;
  private editGeneration = 0;
  private chain: Promise<void> = Promise.resolve();
  private loading: Promise<void> | null = null;

  get ready(): boolean {
    return this.value !== null;
  }

  load(): Promise<void> {
    this.loading ??= api
      .getConfig()
      .then((cfg) => {
        // Edits made before the load finished are impossible (UI waits for `ready`), so just take it.
        this.value = cfg;
      })
      .catch((e: unknown) => {
        this.loading = null;
        toasts.error(e);
      });
    return this.loading;
  }

  /**
   * Mutates the local config and schedules a save. Returns false when the config is not loaded yet.
   * `immediate` skips the debounce (toggles, pickers); typing in inputs should use the default.
   */
  update(mutate: (c: Config) => void, immediate = false): boolean {
    const current = this.value;
    if (!current) return false;
    mutate(current);
    this.editGeneration++;
    const local = validateConfig(current);
    this.fieldErrors = local;
    this.cancelTimer();
    if (Object.keys(local).length > 0) return true;
    if (immediate) void this.saveNow();
    else this.timer = setTimeout(() => void this.saveNow(), SAVE_DEBOUNCE_MS);
    return true;
  }

  /**
   * Takes fields of a config the shell changed and stored by itself (e.g. `resolve_close`). Only `keys` are copied,
   * so local edits that are not saved yet stay; nothing is saved again (the shell already stored it).
   */
  adopt(saved: Config, keys: readonly (keyof Config)[]): void {
    const current = this.value;
    if (!current) {
      this.value = saved;
      return;
    }
    for (const key of keys) copyField(current, saved, key);
  }

  /**
   * Applies a change the shell already stored by itself (e.g. `disconnect_calendar("microsoft")` turning
   * `calendar.microsoft` off). Nothing is saved again; other local edits stay.
   */
  applyStored(mutate: (c: Config) => void): void {
    if (this.value) mutate(this.value);
  }

  /** Saves pending edits right away. Resolves true when the config is valid and stored. */
  async flush(): Promise<boolean> {
    if (!this.value) {
      await this.load();
      return this.value !== null;
    }
    if (Object.keys(validateConfig(this.value)).length > 0) {
      this.fieldErrors = validateConfig(this.value);
      return false;
    }
    if (this.timer !== null) {
      this.cancelTimer();
      await this.saveNow();
    } else {
      await this.chain;
    }
    return Object.keys(this.fieldErrors).length === 0;
  }

  private cancelTimer(): void {
    if (this.timer !== null) clearTimeout(this.timer);
    this.timer = null;
  }

  /** Saves are serialized; a response is applied only if no newer edit happened meanwhile. */
  private saveNow(): Promise<void> {
    this.chain = this.chain.then(async () => {
      const current = this.value;
      if (!current) return;
      const generation = this.editGeneration;
      const payload = $state.snapshot(current) as Config;
      this.saving = true;
      try {
        const saved = await api.updateConfig(payload);
        if (generation === this.editGeneration) {
          this.value = saved;
          this.fieldErrors = {};
        }
        this.savedCount++;
      } catch (e) {
        const err = api.toAppError(e);
        if (err.field) this.fieldErrors = { ...this.fieldErrors, [err.field]: errorText(err) };
        // Also as a toast: not every field has an inline error slot on the current page.
        toasts.error(err);
      } finally {
        this.saving = false;
      }
    });
    return this.chain;
  }
}

export const config = new ConfigStore();
