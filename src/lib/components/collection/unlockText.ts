// Texts of the beta collection's cards and mixer rows (ADR-024): the unlock condition, the progress line and the
// window of a seasonal item. Pure functions over the IPC view and the texts (`strings.collection`); dates are formatted
// with Intl in the language in use. Never a countdown: a window is named by its month or by its first and last day.
import type { Strings } from "$lib/i18n/pl";
import type { AmbienceUnlock, SeasonWindow, ThemeUnlock, UnlockRule, UnlocksView } from "$lib/types";

type C = Strings["collection"];

/** `YYYY-MM-DD` → a local Date at noon (no time-zone edge), or null when malformed. */
function parseDay(day: string): Date | null {
  const m = /^(\d{4})-(\d{2})-(\d{2})$/.exec(day);
  if (!m) return null;
  return new Date(Number(m[1]), Number(m[2]) - 1, Number(m[3]), 12);
}

/** "29 marca" / "March 29" in `locale`. */
export function dayMonth(day: string, locale: string): string {
  const date = parseDay(day);
  return date ? new Intl.DateTimeFormat(locale, { day: "numeric", month: "long" }).format(date) : day;
}

/** "13 października 2026" in `locale` (the day something was earned). */
export function fullDate(day: string, locale: string): string {
  const date = parseDay(day);
  return date ? new Intl.DateTimeFormat(locale, { day: "numeric", month: "long", year: "numeric" }).format(date) : day;
}

/** "Dostępny w październiku" / "Dostępny od 29 marca do 30 kwietnia". */
export function windowText(window: SeasonWindow, c: C, locale: string): string {
  const month = window.month;
  if (month !== null && month >= 1 && month <= 12) return c.availableIn(c.monthIn[month - 1] ?? "");
  return c.availableBetween(dayMonth(window.start, locale), dayMonth(window.end, locale));
}

/** "w tym październiku" / "w tym okresie": the open window in a progress line. */
export function periodText(window: SeasonWindow | null, c: C): string {
  const month = window?.month ?? null;
  if (month !== null && month >= 1 && month <= 12) return c.thisMonth[month - 1] ?? c.thisPeriod;
  return c.thisPeriod;
}

/** The rule in words ("Poziom 3", "Seria 7 dni (wystarczy raz)", "Cel dnia w 3 różne dni", "Razem z motywem …"). */
export function ruleText(rule: UnlockRule, c: C): string {
  switch (rule.type) {
    case "free":
      return c.rule.free;
    case "level":
      return c.rule.level(rule.level);
    case "bestStreak":
      return c.rule.bestStreak(rule.days);
    case "seasonal":
      return c.rule.seasonal(rule.goalDays);
    case "withTheme":
      return c.rule.withTheme(c.themeNames[rule.theme]);
  }
}

/** The progress line of an item that is not unlocked ("Poziom 4 z 6", "2/3 w tym październiku"), or null. */
export function progressText(item: ThemeUnlock | AmbienceUnlock, c: C, window: SeasonWindow | null = null): string | null {
  const p = item.progress;
  if (!p) return null;
  switch (item.rule.type) {
    case "level":
      return c.progressLevel(p.have, p.need);
    case "bestStreak":
      return c.progressStreak(p.have, p.need);
    case "seasonal":
      return c.progressSeason(p.have, p.need, periodText(window ?? ("window" in item ? item.window : null), c));
    default:
      return null;
  }
}

/** The seasonal window that decides a `withTheme` ambience (its theme's), or the theme's own. */
export function themeWindow(unlocks: UnlocksView | null, item: AmbienceUnlock): SeasonWindow | null {
  if (item.rule.type !== "withTheme") return null;
  const theme = item.rule.theme;
  return unlocks?.themes.find((t) => t.theme === theme)?.window ?? null;
}

/**
 * Every line that explains how a locked item is earned, in reading order: the rule, then the progress or the window
 * ("Dostępny w październiku"). `availableNow` items show the rule and the progress of the open window.
 */
export function conditionLines(
  item: ThemeUnlock | AmbienceUnlock,
  c: C,
  locale: string,
  unlocks: UnlocksView | null,
): string[] {
  const lines = [ruleText(item.rule, c)];
  const window = "window" in item ? item.window : themeWindow(unlocks, item);
  if (item.state === "availableNow") {
    const progress = progressText(item, c, window);
    if (progress) lines.push(progress);
    return lines;
  }
  if (item.rule.type === "withTheme") {
    // The ambience comes with its theme: the theme's progress while its window is open, else its window.
    const themeId = item.rule.theme;
    const theme = unlocks?.themes.find((t) => t.theme === themeId) ?? null;
    if (theme?.state === "availableNow") {
      const progress = progressText(theme, c, theme.window);
      if (progress) lines.push(progress);
    } else if (window) {
      lines.push(windowText(window, c, locale));
    }
    return lines;
  }
  if (window && !window.open) lines.push(windowText(window, c, locale));
  const progress = progressText(item, c, window);
  if (progress && item.rule.type !== "seasonal") lines.push(progress);
  return lines;
}
