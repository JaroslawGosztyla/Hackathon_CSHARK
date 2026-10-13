// Calendars: meetings of the next hours, their collisions with the planned breaks and the user's choice per meeting.
// The status is pushed by the shell (`calendar://changed`) and revision-guarded here too, because the commands
// below also return a status. ICS links are secrets: they are read only on demand by Settings (component state,
// never kept here) and never appear in errors, toasts or logs.
import * as api from "$lib/api";
import { errorText } from "$lib/strings";
import type { AppError, CalendarSource, CalendarSourceStatus, CalendarStatus, Config, MeetingChoice } from "$lib/types";
import { config } from "./config.svelte";
import { toasts } from "./toasts.svelte";

/** Anything that looks like a link (`https://…`, `webcal://…`, `www.…`); cut out of every text shown to the user. */
const LINK = /\b(?:[a-z][a-z0-9+.-]*:\/\/|www\.)\S+/gi;

/** Removes links from a (shell / server) detail before it is shown. */
export function redactLinks(text: string): string {
  return text.replace(LINK, "[…]");
}

/** Shows a calendar error as a Polish toast. Unlike `toasts.error` it never logs the detail and strips links from it. */
export function reportCalendarError(e: unknown): void {
  const err: AppError = api.toAppError(e);
  if (err.code === api.ERR_NO_SHELL) return;
  const detail = redactLinks(err.message).trim();
  toasts.push(errorText(err), "error", detail || null);
}

/** Start of the shell's error for a sign-in without calendar permission (focus-platform `reconnect_needed`). */
const RECONNECT_NEEDED = /\breconnect needed\b/i;

/**
 * The source failed because its sign-in lacks the calendar permission (a Microsoft sign-in made for Teams only, or
 * a Google sign-in that expired): signing in again fixes it, nothing has to be disconnected.
 */
export function needsReconnect(source: CalendarSourceStatus | null): boolean {
  return source?.state === "error" && RECONNECT_NEEDED.test(source.detail ?? "");
}

/** At least one calendar integration is switched on in the config. */
export function calendarEnabled(cfg: Config | null): boolean {
  const c = cfg?.calendar;
  return c !== undefined && (c.microsoft || c.google || c.ics);
}

/**
 * A line of the ICS textarea that can be sent: https or webcal(s) with something after `://`, no spaces (the shell
 * refuses plain http: the link is a secret).
 */
const ICS_LINK = /^(?:https|webcals?):\/\/[^\s/]+\S*$/i;

/**
 * Splits the ICS textarea into links (trimmed, empty lines skipped) and the 1-based numbers of lines that do not
 * look like a calendar link. The lines themselves are never echoed back in messages.
 */
export function parseIcsLines(text: string): { urls: string[]; invalidLines: number[] } {
  const urls: string[] = [];
  const invalidLines: number[] = [];
  text.split(/\r?\n/).forEach((raw, i) => {
    const line = raw.trim();
    if (line === "") return;
    if (ICS_LINK.test(line)) urls.push(line);
    else invalidLines.push(i + 1);
  });
  return { urls, invalidLines };
}

class CalendarStore {
  status = $state.raw<CalendarStatus | null>(null);
  /** Choice sent for a meeting id and not answered yet; shown right away and blocks a second click. */
  pending = $state<Record<string, MeetingChoice>>({});
  refreshing = $state(false);
  /** `connect_google` is being called (the shell then reports `googlePending` itself). */
  googleStarting = $state(false);

  apply(status: CalendarStatus): void {
    const current = this.status;
    if (current && !api.isNewerRevision(status.revision, current.revision)) return;
    this.status = status;
  }

  source(source: CalendarSource): CalendarSourceStatus | null {
    return this.status?.sources.find((s) => s.source === source) ?? null;
  }

  /** Reads the current status again (planned breaks move with the clock while idle). Failures stay quiet. */
  async reload(): Promise<void> {
    try {
      this.apply(await api.getCalendar());
    } catch {
      // The next `calendar://changed` brings it anyway.
    }
  }

  /** Fetches all calendars now (pending config edits are saved first, so the shell reads the current sources). */
  async refresh(): Promise<void> {
    if (this.refreshing) return;
    this.refreshing = true;
    try {
      if (!(await config.flush())) return;
      this.apply(await api.refreshCalendar());
    } catch (e) {
      reportCalendarError(e);
    } finally {
      this.refreshing = false;
    }
  }

  /** Stores how a colliding meeting is handled. Resolves true when the shell accepted it. */
  async choose(meetingId: string, choice: MeetingChoice): Promise<boolean> {
    if (meetingId in this.pending) return false;
    this.pending[meetingId] = choice;
    try {
      this.apply(await api.setMeetingChoice(meetingId, choice));
      return true;
    } catch (e) {
      reportCalendarError(e);
      return false;
    } finally {
      delete this.pending[meetingId];
    }
  }

  /** Opens the Google sign-in in the browser; progress (`googlePending`, connected, error) arrives as a status. */
  async connectGoogle(): Promise<void> {
    if (this.googleStarting) return;
    this.googleStarting = true;
    try {
      // The shell reads the client id from the stored config.
      if (!(await config.flush())) return;
      await api.connectGoogle();
    } catch (e) {
      reportCalendarError(e);
    } finally {
      this.googleStarting = false;
    }
  }

  /**
   * Google / ICS: forgets the stored sign-in / links. Microsoft: switches the calendar off but keeps the Teams sign-in
   * (the shell stores `calendar.microsoft = false` itself; pending edits are saved first so they cannot undo it).
   */
  async disconnect(source: CalendarSource): Promise<boolean> {
    try {
      if (source === "microsoft" && !(await config.flush())) return false;
      this.apply(await api.disconnectCalendar(source));
      if (source === "microsoft") config.applyStored((c) => (c.calendar.microsoft = false));
      return true;
    } catch (e) {
      reportCalendarError(e);
      return false;
    }
  }

  /** The stored ICS links, for the Settings editor only. Resolves null when they could not be read. */
  async loadIcsUrls(): Promise<string[] | null> {
    try {
      return await api.getIcsUrls();
    } catch (e) {
      reportCalendarError(e);
      return null;
    }
  }

  /** Replaces the ICS links. Resolves true when the shell stored them. */
  async saveIcsUrls(urls: string[]): Promise<boolean> {
    try {
      this.apply(await api.setIcsUrls(urls));
      return true;
    } catch (e) {
      reportCalendarError(e);
      return false;
    }
  }
}

export const calendar = new CalendarStore();
