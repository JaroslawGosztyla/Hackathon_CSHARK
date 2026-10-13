// Texts of the Developer page (ADR-019). English only, on purpose: a developer tool for tests and screenshots, not a
// part of the product, so these are plain constants outside the `Strings` type (exception to ADR-014 point 5).
// The Settings switch and the nav tab label are product texts: they live in the 10 locale files. Sample content that
// shows outside the Developer page (meetings, the break reminder, the sample-link toast) follows the UI language
// (`sampleTexts`).
import type { MascotMood, MascotReaction } from "$lib/mascot/frames";
import type { Phase, PopupKind } from "$lib/types";
import { de } from "./de";
import { en } from "./en";
import { es } from "./es";
import { fr } from "./fr";
import { it } from "./it";
import { ja } from "./ja";
import { pl, type Strings } from "./pl";
import { ru } from "./ru";
import { uk } from "./uk";
import { zh } from "./zh";
import { language, type UiLanguage } from "./index.svelte";

/** One popup button: the kind and, for a meeting reminder, how many minutes before the meeting it comes. */
export interface PopupSample {
  kind: PopupKind;
  leadMinutes: number | null;
  label: string;
}

/** Every popup the shell can show, in the order of the buttons and of "Play all popups". */
export const POPUP_SAMPLES: readonly PopupSample[] = [
  { kind: "focusStarted", leadMinutes: null, label: "Focus started" },
  { kind: "breakStarted", leadMinutes: null, label: "Break started (with reminder)" },
  { kind: "interrupted", leadMinutes: null, label: "Interrupted" },
  { kind: "launchersClosed", leadMinutes: null, label: "Launchers closed" },
  { kind: "meetingReminder", leadMinutes: 30, label: "Meeting reminder · 30 min" },
  { kind: "meetingReminder", leadMinutes: 20, label: "Meeting reminder · 20 min" },
  { kind: "meetingReminder", leadMinutes: 10, label: "Meeting reminder · 10 min (no “Remind later”)" },
  { kind: "breakHeld", leadMinutes: null, label: "Break held" },
  { kind: "workDigest", leadMinutes: null, label: "Work digest" },
];

export const TOAST_SAMPLES: readonly { phase: Exclude<Phase, "idle">; label: string }[] = [
  { phase: "focus", label: "Focus toast" },
  { phase: "break", label: "Break toast" },
];

/** "Phase ends in …" buttons (seconds). */
export const REMAINING_SAMPLES: readonly { secs: number; label: string }[] = [
  { secs: 5, label: "Phase ends in 5 s" },
  { secs: 30, label: "Phase ends in 30 s" },
  { secs: 120, label: "Phase ends in 2 min" },
];

/** "Jump to 12:34 left": a ring that looks mid-session in a screenshot. */
export const RING_SAMPLE_SECS = 12 * 60 + 34;

/** Warm screen preview strengths (%, `preview_tint` takes 0..=100). */
export const TINT_PREVIEW_LEVELS: readonly number[] = [0, 25, 50, 75, 100];

export const MASCOT_REACTIONS: readonly { reaction: MascotReaction; label: string }[] = [
  { reaction: "start", label: "Start · wink" },
  { reaction: "break", label: "Break · smile" },
  { reaction: "cycle", label: "Cycle done · happy" },
  { reaction: "nudge", label: "Blocked site · angry" },
  { reaction: "stopEarly", label: "Early stop · sad" },
  { reaction: "meeting", label: "Meeting · smile" },
  { reaction: "digest", label: "Work digest · happy" },
];

export const MASCOT_MOODS: readonly { mood: MascotMood; label: string }[] = [
  { mood: "neutral", label: "Neutral" },
  { mood: "calm", label: "Calm" },
  { mood: "smile", label: "Smile" },
  { mood: "sad", label: "Sad" },
  { mood: "angry", label: "Angry" },
  { mood: "happy", label: "Happy" },
];

const LOCALE_TEXTS: Record<UiLanguage, Strings> = { en, pl, de, fr, it, uk, ru, es, zh, ja };

/** A line in `lang` itself (its own script), from that locale's mascot lines: shows the lip-sync per language. */
export function speechSample(lang: UiLanguage): string {
  const m = LOCALE_TEXTS[lang].mascot;
  return [m.startLines[0], m.cycleLines[0]].filter((line): line is string => typeof line === "string").join(" ");
}

/** Plain English texts of the Developer page. */
export const DEV = {
  title: "Developer",
  lead: "Test triggers for screenshots and demos. English only; samples live in memory and vanish on “Clear samples” or a restart, and are never saved or sent anywhere. Look & feel, Window & app and “Show the mascot” change real settings, like Settings.",
  off: "Developer mode is off.",
  offLink: "Turn it on in Settings",

  cleanScreenshots: "Clean screenshots",
  cleanScreenshotsHint: "Hides the Developer tab in the navigation. Ctrl+Shift+D (or Settings) brings it back.",
  keepPopupOpen: "Keep popup open",
  keepPopupOpenHint: "Sample popups stay until closed: no countdown, no auto-hide.",
  statusIdle: "Ready. Every button does one thing; results show here.",

  popupsTitle: "Popups",
  popupsHint: "The real corner popup with sample content from the shell. Shows at once, even while presenting.",
  playAll: (seconds: number) => `Play all popups (every ${seconds} s)`,
  playing: (step: number, total: number) => `Playing ${step} / ${total}…`,
  stop: "Stop",
  dismissPopup: "Dismiss popup",

  toastsTitle: "Windows toasts",
  toastsHint: "The same Windows notification a real phase change shows. Do not disturb may silence it.",

  sessionTitle: "Session",
  sessionHint: "A real session: what you do here ends up in the history like any other session.",
  start: "Start",
  pause: "Pause",
  resume: "Resume",
  skip: "Skip phase",
  stopSession: "Stop",
  ringSample: "Jump to 12:34 left",
  idleState: "Idle",
  sessionState: (phase: string, cycle: number, left: string, paused: boolean) =>
    `${phase} · cycle ${cycle} · ${left} left${paused ? " · paused" : ""}`,
  needsSession: "Start a session first.",

  mascotTitle: "Mascot",
  mascotHint: "Opens Focus, then plays. Held moods stay until “Back to automatic mood”.",
  mascotOff: "The mascot is hidden in Settings.",
  mascotOn: "Show the mascot",
  reactionsLabel: "Reactions",
  tickle: "Tickle + line",
  moodsLabel: "Hold a mood",
  autoMood: "Back to automatic mood",
  speakLabel: "Speak",
  speakShort: "Short line",
  speakLong: "Long scrolling text",
  speakBreak: "Break reminder + tip",
  languagesLabel: "One line in each language",
  shortcutLabel: "Shortcut",
  pixelLook: "Force 8-bit theme look",

  bannersTitle: "Banners & dialogs",
  bannersHint: "Shown on Focus, from memory only.",
  nudgeBanner: "Blocked-site banner",
  errorToast: "App error toast",
  infoToast: "App info toast",
  closePrompt: "Close-to-tray prompt",
  holdExtend: "Meeting hold banner · extend",
  holdPause: "Meeting hold banner · pause at break",
  goalLine: "“What's next?” goal line",

  samplesTitle: "Sample data",
  samplesHint: "In memory only: never written to history.jsonl or meeting-choices.json, no calendar or tool is contacted.",
  sampleSummary: "Summary chart + history",
  sampleWork: "“What's next?” · 5 items",
  sampleCalendar: "Calendar conflicts · 2 meetings",
  clearSamples: "Clear samples",
  activeSamples: "Active:",
  noSamples: "No samples shown.",
  sampleNames: {
    summary: "summary",
    work: "what's next",
    calendar: "calendar",
    goal: "goal",
    hold: "meeting hold",
    mood: "held mood",
  },

  lookTitle: "Look & feel",
  lookHint: "Themes and languages change the real setting (like Settings).",
  themesLabel: "Theme",
  /** The page stays English whatever the UI language: the English theme and phase names. */
  themeNames: en.themes.names,
  phaseNames: en.phase,
  languagesPick: "Language",
  systemLanguage: "System",
  tintLabel: "Warm screen preview",
  tintLevel: (percent: number) => `${percent} %`,
  tintStop: "Stop preview",
  chimesLabel: "Chimes",
  chimesHint: "Plays each sound once; your chosen sound is restored afterwards.",
  chimeBreak: "Break",
  chimeFocus: "Focus",
  chimeNames: {
    midnight: "Midnight",
    vaporwave: "Vaporwave",
    lunar: "Lunar",
    autumn: "Autumn",
    classic: "Classic",
    soft: "Soft",
    chiptune: "Chiptune",
    custom: "Custom file",
  },

  appTitle: "Window & app",
  appHint: "Housekeeping for repeated takes.",
  resetClose: "Reset first-close choice",
  resetCloseDone: "The next X on the main window asks again.",
  reloadUi: "Reload UI",

  // Status lines
  sent: (what: string) => `${what}: done.`,
  failed: (what: string, code: string, message: string) => `${what} failed (${code}): ${message}`,
  samplesCleared: "Samples cleared.",
  sampleClosePrompt: "Sample prompt: the buttons only close it.",

  // Technical detail of the sample error toast (shell error details are English too)
  sampleErrorDetail: "sample error from the Developer page",
} as const;

/** Sample content shown outside the Developer page (Skupienie, Podsumowanie), in the UI language. */
export interface SampleTexts {
  /** The sample meeting of the hold banner and the first calendar conflict (the shell's sample popups: `dev_sample_meeting`). */
  meeting: string;
  /** Two more meetings of the sample calendar. */
  meetingOneOnOne: string;
  meetingDesign: string;
  /** The sample break reminder (the shell's sample popups: `dev_sample_reminder`). */
  reminder: string;
  /** Info toast when a sample item's link is clicked. */
  linkNotOpened: string;
}

// Meeting and reminder texts mirror `dev_sample_meeting` / `dev_sample_reminder` in `src-tauri/src/texts/*.rs`, so a
// sample popup and the main window show the same meeting in one screenshot.
const SAMPLE_TEXTS: Record<UiLanguage, SampleTexts> = {
  en: {
    meeting: "Sprint planning",
    meetingOneOnOne: "1:1 with Alex",
    meetingDesign: "Design critique",
    reminder: "Water and a short walk",
    linkNotOpened: "Sample item: the link is not opened.",
  },
  pl: {
    meeting: "Planowanie sprintu",
    meetingOneOnOne: "1:1 z Alexem",
    meetingDesign: "Przegląd designu",
    reminder: "Woda i krótki spacer",
    linkNotOpened: "Przykładowa pozycja: link się nie otwiera.",
  },
  de: {
    meeting: "Sprint-Planung",
    meetingOneOnOne: "1:1 mit Alex",
    meetingDesign: "Design-Review",
    reminder: "Wasser und ein kurzer Spaziergang",
    linkNotOpened: "Beispieleintrag: Der Link wird nicht geöffnet.",
  },
  fr: {
    meeting: "Planification du sprint",
    meetingOneOnOne: "Point 1:1 avec Alex",
    meetingDesign: "Revue de design",
    reminder: "De l'eau et une petite marche",
    linkNotOpened: "Élément d'exemple : le lien ne s'ouvre pas.",
  },
  it: {
    meeting: "Pianificazione dello sprint",
    meetingOneOnOne: "1:1 con Alex",
    meetingDesign: "Revisione del design",
    reminder: "Acqua e una breve passeggiata",
    linkNotOpened: "Elemento di esempio: il link non viene aperto.",
  },
  uk: {
    meeting: "Планування спринту",
    meetingOneOnOne: "1:1 з Алексом",
    meetingDesign: "Огляд дизайну",
    reminder: "Вода й коротка прогулянка",
    linkNotOpened: "Приклад: посилання не відкривається.",
  },
  ru: {
    meeting: "Планирование спринта",
    meetingOneOnOne: "1:1 с Алексом",
    meetingDesign: "Разбор дизайна",
    reminder: "Вода и короткая прогулка",
    linkNotOpened: "Пример: ссылка не открывается.",
  },
  es: {
    meeting: "Planificación del sprint",
    meetingOneOnOne: "1:1 con Alex",
    meetingDesign: "Revisión de diseño",
    reminder: "Agua y un paseo corto",
    linkNotOpened: "Elemento de ejemplo: el enlace no se abre.",
  },
  zh: {
    meeting: "冲刺规划会",
    meetingOneOnOne: "与 Alex 一对一",
    meetingDesign: "设计评审",
    reminder: "喝水，短暂散步",
    linkNotOpened: "示例条目：不会打开链接。",
  },
  ja: {
    meeting: "スプリント計画",
    meetingOneOnOne: "Alex との 1on1",
    meetingDesign: "デザインレビュー",
    reminder: "水分補給と短い散歩",
    linkNotOpened: "サンプル項目：リンクは開きません。",
  },
};

/** Sample content in the current UI language (read when a sample is built, i.e. at click time). */
export function sampleTexts(): SampleTexts {
  return SAMPLE_TEXTS[language()];
}
