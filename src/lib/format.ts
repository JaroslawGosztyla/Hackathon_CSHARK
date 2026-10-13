// Formatting helpers for times, dates and process names (Polish locale).
import { strings } from "./strings";

const LOCALE = "pl-PL";

const pad2 = (n: number) => String(n).padStart(2, "0");

/** Countdown clock: `MM:SS` (minutes are not wrapped into hours, so 90 min shows as `90:00`). */
export function formatClock(totalSecs: number): string {
  const secs = Math.max(0, Math.floor(totalSecs));
  return `${pad2(Math.floor(secs / 60))}:${pad2(secs % 60)}`;
}

/** Human duration: `1 h 5 min`, `25 min`, `40 s`. */
export function formatDuration(totalSecs: number): string {
  const secs = Math.max(0, Math.round(totalSecs));
  if (secs < 60) return strings.time.seconds(secs);
  const totalMinutes = Math.round(secs / 60);
  const h = Math.floor(totalMinutes / 60);
  const m = totalMinutes % 60;
  if (h === 0) return strings.time.minutes(m);
  return m === 0 ? strings.time.hours(h) : strings.time.hoursMinutes(h, m);
}

const timeFmt = new Intl.DateTimeFormat(LOCALE, { hour: "2-digit", minute: "2-digit" });
const dateFmt = new Intl.DateTimeFormat(LOCALE, { day: "numeric", month: "short", year: "numeric" });
const shortDateFmt = new Intl.DateTimeFormat(LOCALE, { day: "numeric", month: "short" });
const pctFmt = new Intl.NumberFormat(LOCALE, { style: "percent", maximumFractionDigits: 0 });

export const formatTimeOfDay = (ms: number) => timeFmt.format(new Date(ms));
export const formatDate = (ms: number) => dateFmt.format(new Date(ms));
export const formatShortDate = (ms: number) => shortDateFmt.format(new Date(ms));

/** `12 paź 2026, 09:15–10:45` (end date omitted when on the same day). */
export function formatSpan(startMs: number, endMs: number): string {
  const sameDay = new Date(startMs).toDateString() === new Date(endMs).toDateString();
  const end = sameDay ? formatTimeOfDay(endMs) : `${formatDate(endMs)}, ${formatTimeOfDay(endMs)}`;
  return `${formatDate(startMs)}, ${formatTimeOfDay(startMs)}–${end}`;
}

/** Fraction 0..1 → `54%`. Values that round to 0 but are not 0 show as `<1%`. */
export function formatPercent(fraction: number): string {
  if (!Number.isFinite(fraction) || fraction <= 0) return pctFmt.format(0);
  if (fraction < 0.005) return `<${pctFmt.format(0.01)}`;
  return pctFmt.format(fraction);
}

/** `Code.exe` → `Code`; keeps other names as they are. */
export function processLabel(process: string): string {
  return process.replace(/\.exe$/i, "");
}

const MINUTES_PER_DAY = 24 * 60;

/** Minutes after midnight → `HH:MM` (the value format of `<input type="time">`). */
export function minutesToTime(minutes: number): string {
  const m = ((Math.trunc(minutes) % MINUTES_PER_DAY) + MINUTES_PER_DAY) % MINUTES_PER_DAY;
  return `${pad2(Math.floor(m / 60))}:${pad2(m % 60)}`;
}

/** `HH:MM` (or `HH:MM:SS`) from `<input type="time">` → minutes after midnight; null when empty or malformed. */
export function timeToMinutes(value: string): number | null {
  const match = /^(\d{1,2}):(\d{2})(?::\d{2}(?:\.\d+)?)?$/.exec(value.trim());
  if (!match) return null;
  const h = Number(match[1]);
  const m = Number(match[2]);
  if (h > 23 || m > 59) return null;
  return h * 60 + m;
}
