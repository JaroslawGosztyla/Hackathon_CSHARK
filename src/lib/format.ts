// Formatting helpers for times, dates and process names, in the Intl locale of the language in use
// (`intlLocale` is reactive, so formatted values re-render when the language changes).
import { intlLocale, strings } from "./i18n/index.svelte";

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

const DATE_OPTIONS = {
  time: { hour: "2-digit", minute: "2-digit" },
  date: { day: "numeric", month: "short", year: "numeric" },
  shortDate: { day: "numeric", month: "short" },
} satisfies Record<string, Intl.DateTimeFormatOptions>;

const dateFormats = new Map<string, Intl.DateTimeFormat>();
const percentFormats = new Map<string, Intl.NumberFormat>();

/** One cached formatter per locale and kind (creating Intl formatters is slow). */
function dateFormat(kind: keyof typeof DATE_OPTIONS): Intl.DateTimeFormat {
  const locale = intlLocale();
  const key = `${locale}|${kind}`;
  let format = dateFormats.get(key);
  if (!format) {
    format = new Intl.DateTimeFormat(locale, DATE_OPTIONS[kind]);
    dateFormats.set(key, format);
  }
  return format;
}

function percentFormat(): Intl.NumberFormat {
  const locale = intlLocale();
  let format = percentFormats.get(locale);
  if (!format) {
    format = new Intl.NumberFormat(locale, { style: "percent", maximumFractionDigits: 0 });
    percentFormats.set(locale, format);
  }
  return format;
}

export const formatTimeOfDay = (ms: number) => dateFormat("time").format(new Date(ms));
export const formatDate = (ms: number) => dateFormat("date").format(new Date(ms));
export const formatShortDate = (ms: number) => dateFormat("shortDate").format(new Date(ms));

/** `12 paź 2026, 09:15–10:45` (end date omitted when on the same day). */
export function formatSpan(startMs: number, endMs: number): string {
  const sameDay = new Date(startMs).toDateString() === new Date(endMs).toDateString();
  const end = sameDay ? formatTimeOfDay(endMs) : `${formatDate(endMs)}, ${formatTimeOfDay(endMs)}`;
  return `${formatDate(startMs)}, ${formatTimeOfDay(startMs)}–${end}`;
}

/** Whole local calendar days from the day of `nowMs` to the day of `ms` (0 = same day, 1 = tomorrow). */
function dayOffset(ms: number, nowMs: number): number {
  const a = new Date(ms);
  const b = new Date(nowMs);
  const dayA = Date.UTC(a.getFullYear(), a.getMonth(), a.getDate());
  const dayB = Date.UTC(b.getFullYear(), b.getMonth(), b.getDate());
  return Math.round((dayA - dayB) / 86_400_000);
}

/** Meeting / break time: `11:00–11:30`; another day than today gets a prefix (`jutro 09:00–09:30`, `14 paź 09:00–09:30`). */
export function formatTimeRange(startMs: number, endMs: number, nowMs: number = Date.now()): string {
  const range = `${formatTimeOfDay(startMs)}–${formatTimeOfDay(endMs)}`;
  const offset = dayOffset(startMs, nowMs);
  if (offset === 0) return range;
  if (offset === 1) return `${strings.calendar.tomorrow} ${range}`;
  return `${formatShortDate(startMs)} ${range}`;
}

/** A time of day that may be on another day than today: `11:30`, `jutro 09:00`. */
export function formatTimeNear(ms: number, nowMs: number = Date.now()): string {
  const offset = dayOffset(ms, nowMs);
  if (offset === 0) return formatTimeOfDay(ms);
  if (offset === 1) return `${strings.calendar.tomorrow} ${formatTimeOfDay(ms)}`;
  return `${formatShortDate(ms)} ${formatTimeOfDay(ms)}`;
}

/** A day that may be today: `dziś`, `jutro`, `14 paź`. */
export function formatDayNear(ms: number, nowMs: number = Date.now()): string {
  const offset = dayOffset(ms, nowMs);
  if (offset === 0) return strings.time.today;
  if (offset === 1) return strings.calendar.tomorrow;
  return formatShortDate(ms);
}

/** Fraction 0..1 → `54%`. Values that round to 0 but are not 0 show as `<1%`. */
export function formatPercent(fraction: number): string {
  const pct = percentFormat();
  if (!Number.isFinite(fraction) || fraction <= 0) return pct.format(0);
  if (fraction < 0.005) return `<${pct.format(0.01)}`;
  return pct.format(fraction);
}

// Weekly stats (ADR-023): weekday names, week ranges and hours of the day in the Intl locale of the language in use.

/** A Monday (2024-01-01), so `WEEKDAY_ANCHOR + i` days is weekday `i` (0 = Monday). */
const WEEKDAY_ANCHOR = { year: 2024, month: 0, day: 1 } as const;
const weekdayFormats = new Map<string, Intl.DateTimeFormat>();
const rangeFormats = new Map<string, Intl.DateTimeFormat>();

/** `pon.` / `poniedziałek` for weekday index `i` (0 = Monday … 6 = Sunday). */
export function formatWeekday(i: number, style: "short" | "long"): string {
  const locale = intlLocale();
  const key = `${locale}|${style}`;
  let format = weekdayFormats.get(key);
  if (!format) {
    format = new Intl.DateTimeFormat(locale, { weekday: style });
    weekdayFormats.set(key, format);
  }
  return format.format(new Date(WEEKDAY_ANCHOR.year, WEEKDAY_ANCHOR.month, WEEKDAY_ANCHOR.day + i));
}

/** `12–18 paź 2026` (both ends local midnights in Unix ms; the month and year are not repeated when shared). */
export function formatDayRange(startMs: number, endMs: number): string {
  const locale = intlLocale();
  let format = rangeFormats.get(locale);
  if (!format) {
    format = new Intl.DateTimeFormat(locale, DATE_OPTIONS.date);
    rangeFormats.set(locale, format);
  }
  return format.formatRange(new Date(startMs), new Date(endMs));
}

/** Local hour `h` (0..23) as a time of day in the locale: `09:00`. */
export function formatHour(h: number): string {
  return dateFormat("time").format(new Date(WEEKDAY_ANCHOR.year, WEEKDAY_ANCHOR.month, WEEKDAY_ANCHOR.day, h));
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

const sizeFormats = new Map<string, Intl.NumberFormat>();

/** Size on disk in KiB → `840 kB`, `12 MB`, `1.4 GB` in the Intl locale (1 KiB shown as 1 kB; at most 1 decimal). */
export function formatSize(kib: number): string {
  const units = ["kilobyte", "megabyte", "gigabyte"] as const;
  let value = Math.max(0, kib);
  let unit = 0;
  while (value >= 1024 && unit < units.length - 1) {
    value /= 1024;
    unit++;
  }
  const locale = intlLocale();
  const key = `${locale}|${units[unit]}`;
  let format = sizeFormats.get(key);
  if (!format) {
    format = new Intl.NumberFormat(locale, {
      style: "unit",
      unit: units[unit],
      unitDisplay: "short",
      maximumFractionDigits: unit === 0 ? 0 : 1,
    });
    sizeFormats.set(key, format);
  }
  return format.format(value);
}
