// Local calendar dates as `YYYY-MM-DD` for the weekly stats (ADR-023): ISO weeks, Monday first, in the time zone of
// this computer (the shell aggregates with the same one). Day numbers count days since 1970-01-01, so date arithmetic
// never meets a daylight-saving change.

export const DAYS_PER_WEEK = 7;
export const HOURS_PER_DAY = 24;

const DAY_MS = 86_400_000;
/** 1970-01-01 (day 0) was a Thursday: weekday index 3 with Monday = 0. */
const EPOCH_WEEKDAY = 3;
/** The Thursday of a week decides its ISO week-numbering year (ISO 8601). */
const ISO_THURSDAY = 3;
const DATE_RE = /^(\d{4})-(\d{2})-(\d{2})$/;

const pad = (n: number, width: number) => String(n).padStart(width, "0");

/** `YYYY-MM-DD` → its day number; null for anything else. */
export function dayOfDate(date: string): number | null {
  const match = DATE_RE.exec(date);
  if (!match) return null;
  const [year, month, day] = [Number(match[1]), Number(match[2]), Number(match[3])];
  const ms = Date.UTC(year, month - 1, day);
  const check = new Date(ms);
  if (check.getUTCFullYear() !== year || check.getUTCMonth() !== month - 1 || check.getUTCDate() !== day) return null;
  return Math.round(ms / DAY_MS);
}

/** Day number → `YYYY-MM-DD`. */
export function dateOfDay(day: number): string {
  const d = new Date(day * DAY_MS);
  return `${pad(d.getUTCFullYear(), 4)}-${pad(d.getUTCMonth() + 1, 2)}-${pad(d.getUTCDate(), 2)}`;
}

/** The local calendar day of an instant (Unix ms) as a day number. */
export function localDayOf(ms: number): number {
  const d = new Date(ms);
  return Math.round(Date.UTC(d.getFullYear(), d.getMonth(), d.getDate()) / DAY_MS);
}

/** Today in local time, `YYYY-MM-DD`. */
export function todayDate(now: number = Date.now()): string {
  return dateOfDay(localDayOf(now));
}

/** 0 = Monday … 6 = Sunday. */
export function weekdayIndex(day: number): number {
  return (((day + EPOCH_WEEKDAY) % DAYS_PER_WEEK) + DAYS_PER_WEEK) % DAYS_PER_WEEK;
}

/** The Monday of the week that contains `day`. */
export function mondayOfDay(day: number): number {
  return day - weekdayIndex(day);
}

/** The Monday (`YYYY-MM-DD`) of the week that contains `date`; the date itself when it is not a date. */
export function mondayOf(date: string): string {
  const day = dayOfDate(date);
  return day === null ? date : dateOfDay(mondayOfDay(day));
}

/** `date` moved by `days` (negative = back). */
export function addDays(date: string, days: number): string {
  const day = dayOfDate(date);
  return day === null ? date : dateOfDay(day + days);
}

/** Local midnight of `date` as Unix ms (for Intl formatting); NaN when it is not a date. */
export function localMidnightMs(date: string): number {
  const match = DATE_RE.exec(date);
  if (!match) return Number.NaN;
  return new Date(Number(match[1]), Number(match[2]) - 1, Number(match[3])).getTime();
}

/** ISO 8601 week-numbering year and week (1..=53) of the week that contains `day`. */
export function isoWeekOfDay(day: number): { year: number; week: number } {
  const thursday = mondayOfDay(day) + ISO_THURSDAY;
  const year = new Date(thursday * DAY_MS).getUTCFullYear();
  const firstDay = Math.round(Date.UTC(year, 0, 1) / DAY_MS);
  return { year, week: Math.floor((thursday - firstDay) / DAYS_PER_WEEK) + 1 };
}
