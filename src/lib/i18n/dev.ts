// Texts of the Developer page (ADR-019). English only, on purpose: a developer tool for tests and screenshots, not a
// part of the product, so these are plain constants outside the `Strings` type (exception to ADR-014 point 5).
// The Settings switch and the nav tab label are product texts: they live in the 10 locale files. Sample content that
// shows outside the Developer page (meetings, the break reminder, the sample-link toast) follows the UI language
// (`sampleTexts`).
import type { MascotMood, MascotReaction } from "$lib/mascot/frames";
import type { RewardMomentSample, RewardsStatusSample } from "$lib/state/samples";
import type { SpookyOneShot } from "$lib/ambience/layers/spookyNight";
import type { AmbienceLayer, Phase, PopupKind, RewardsCheat, RewardTheme } from "$lib/types";
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
  toastsHint:
    "The Windows notification a real phase change shows when FocusMode popups are off (or a popup cannot be shown). Do not disturb may silence it.",

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
  samplesHint:
    "In memory only: never written to history.jsonl, config.json (the sample week's app labels) or meeting-choices.json, no calendar or tool is contacted.",
  sampleSummary: "Summary chart + history",
  sampleWork: "“What's next?” · 5 items",
  sampleCalendar: "Calendar conflicts · 2 meetings",
  sampleSmartBreak: "Smart break · moved break + note",
  sampleTintPause: "Warm colours paused · Photoshop in front",
  sampleWeek: "Sample week · 10 weeks of stats",
  clearSamples: "Clear samples",
  activeSamples: "Active:",
  noSamples: "No samples shown.",
  sampleNames: {
    summary: "summary",
    work: "what's next",
    calendar: "calendar",
    goal: "goal",
    hold: "meeting hold",
    smartBreak: "smart break",
    tint: "tint pause",
    week: "week",
    mood: "held mood",
    themePreview: "collection theme",
  },

  lookTitle: "Look & feel",
  lookHint: "Themes and languages change the real setting (like Settings).",
  themesLabel: "Theme",
  /** The page stays English whatever the UI language: the English theme and phase names. */
  themeNames: en.themes.names,
  rewardThemesLabel: "Collection themes (beta) · sample",
  rewardThemesHint: "Shows the theme on screen whatever its lock state; never saved and never unlocks anything.",
  /** English names of the beta collection themes (ADR-024); the product names live in the locale files. */
  rewardThemeNames: {
    underwater: "Underwater",
    rainyWindow: "Rainy window",
    forestCabin: "Forest cabin",
    sakuraNight: "Sakura night",
    newYear: "New Year",
    valentines: "Valentine's",
    easter: "Easter",
    summer: "Summer",
    halloween: "Halloween",
    christmas: "Christmas",
  } satisfies Record<RewardTheme, string>,
  rewardThemeStop: "Stop theme preview",
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

  presentationMode: (focusSecs: number, breakSecs: number) =>
    `Presentation mode (${focusSecs} s focus / ${breakSecs} s break)`,
  presentationHint:
    "The short timing of --demo, for quick demos. This run only (nothing is saved): a restart is back to your own times unless started with --demo. Presentation sessions are marked “demo”.",
  presentationNow: (on: boolean) => `Presentation mode ${on ? "on" : "off"}.`,
  presentationNextStart: (on: boolean) =>
    `Presentation mode ${on ? "on" : "off"} from the next Start: the running session keeps its lengths.`,
  presentationPending: "Takes effect at the next Start; the running session keeps its lengths.",

  appTitle: "Window & app",
  appHint: "Housekeeping for repeated takes.",
  resetClose: "Reset first-close choice",
  resetCloseDone: "The next X on the main window asks again.",
  showWizard: "Show first-run wizard",
  showWizardHint:
    "From its first step. Its steps change real settings, like Settings; skipping keeps onboardingDone as it is, only finishing stores it.",
  showWizardDone: "First-run wizard shown.",
  reloadUi: "Reload UI",
  factoryReset: "Factory reset…",
  factoryResetHint: "Deletes everything FocusMode keeps on this computer and restarts it like a fresh install.",
  factoryResetTitle: "Factory reset?",
  factoryResetBody:
    "FocusMode deletes all its data, then restarts like a fresh install (Developer mode is off again). This cannot be undone. The installed app, the browser extension folder and your music folders stay.",
  factoryResetSessionStops: "The running session stops first; shields and Windows settings come back before anything goes.",
  /** What goes (the same as Settings → “Delete everything”, plus the settings file itself). */
  factoryResetList: [
    "Settings (config.json): every setting back to the default",
    "Session history and the last summary",
    "Sparks, streak and surprises (beta), when there are any",
    "Meeting answers and seen “What's next?” items",
    "Sign-ins: Microsoft (Teams and calendar), Google Calendar, ICS links and work tool tokens",
    "Imported chime sounds",
    "Your site list (back to the bundled one)",
    "Browser data of the app: YouTube sign-in, cookies and cache",
    "Logs and the launch-on-login entry",
  ],
  factoryResetConfirm: "Reset and restart",
  factoryResetRestarting: "Factory reset done: restarting…",
  /** `tauri dev` only: the restarted app has no dev server. */
  factoryResetDevNote: "Under “npm run tauri dev” the restarted app has no dev server: start it again.",
  /** The action's name in the status line. */
  factoryResetName: "Factory reset",
  cancel: "Cancel",

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

/** The pose gallery (`$lib/mascot/PoseGallery.svelte`): atlas QA for the beta poses (ADR-022). */
export const POSE_GALLERY = {
  title: "Pose gallery (beta)",
  hint: "Atlas QA: the 11 poses next to blink-1 in boxes of the same size, feet on the dashed line. Shown whatever the beta switches say; check Midnight, Light and 8-bit.",
  open: "Show the pose gallery",
  close: "Hide the pose gallery",
  overlay: "Lay blink-1 over each pose",
  size: (px: number) => `${px} px`,
  sizeLabel: "Box height",
  idle: "blink-1 (main atlas)",
  loading: "Loading the poses atlas…",
  failed: "The poses atlas did not load (see the console).",
};

/**
 * Developer page → "Rewards (beta)" (ADR-022). Every button builds a frontend sample (`sample: true`, memory only:
 * never acknowledged, sent or saved) and opens the page where it shows, except the popups (`dev_trigger`, memory only
 * in the shell) and the sounds (`play_reward_sound`).
 */
export const DEV_REWARDS = {
  title: "Rewards (beta)",
  hint: "Samples of the sparks, streak and small-steps surfaces, in memory only. They show even while the beta features are off; the poses need the beta features on (fallback frames otherwise).",
  sampleName: "rewards",
  smallLabel: "Small moments",
  small: [
    { kind: "start", label: "Start (+12, first of day)" },
    { kind: "step", label: "Step ticked" },
    { kind: "firstStep", label: "First step (01)" },
    { kind: "allSteps", label: "All steps done" },
    { kind: "back", label: "Back from break" },
  ] as const satisfies readonly { kind: RewardMomentSample; label: string }[],
  burstsLabel: "Bursts",
  bursts: [
    { kind: "recap", label: "Break recap (plain)" },
    { kind: "bubble", label: "+ bubble" },
    { kind: "pearl", label: "+ pearl" },
    { kind: "golden", label: "+ golden pearl" },
    { kind: "dailyGoal", label: "Daily goal" },
    { kind: "comeback", label: "Comeback" },
    { kind: "streak", label: "Streak +1" },
    { kind: "milestone", label: "Streak milestone 7" },
    { kind: "restUsed", label: "Rest day used" },
    { kind: "restEarned", label: "Rest day earned" },
    { kind: "levelUp", label: "Level up" },
    { kind: "levelUp2", label: "Level up ×2" },
    { kind: "pondFull", label: "Pond full" },
    { kind: "full", label: "Full burst (everything)" },
    { kind: "sessionEnd", label: "Session end" },
    { kind: "earlyStop", label: "Early stop" },
    { kind: "goalDone", label: "Goal done “Tak”" },
    { kind: "sticker", label: "Sticker unlocked" },
    { kind: "unlockHalloween", label: "Unlock · Halloween (theme + sound)" },
    { kind: "unlockUnderwater", label: "Unlock · Underwater (theme)" },
  ] as const satisfies readonly { kind: RewardMomentSample; label: string }[],
  statusLabel: "Status samples",
  statuses: [
    { kind: "today", label: "Today 18/30, streak 6, level 7" },
    { kind: "goalMet", label: "Goal met today" },
    { kind: "waiting", label: "Streak waiting (best 12)" },
    { kind: "first", label: "First day ever" },
    { kind: "paused", label: "Streak paused" },
    { kind: "allMarks", label: "Week strip: every mark" },
    { kind: "preview", label: "Demo preview badge" },
  ] as const satisfies readonly { kind: RewardsStatusSample; label: string }[],
  clear: "Clear reward samples",
  cleared: "Reward samples cleared: the real state shows again.",
  popupsLabel: "Popups",
  popupsHint: "The shell adds the reward line and the next step only while the beta features are on.",
  popupBreak: "Break popup with sparks",
  popupFocus: "Focus popup with next step",
  posesLabel: "Poses",
  posesHint: "Each button holds the pose for 5 s on the Skupienie axolotl. The pose gallery (Mascot section) checks the atlas.",
  pose: (name: string) => `Pose ${name}`,
  moodThinking: "Mood: thinking",
  moodWorking: "Mood: working",
  soundsLabel: "Sounds",
  soundsHint: "Played by the shell: they need the beta features (sparks) and the reward sounds on.",
  sounds: { pop: "Pop", sparkle: "Sparkle", fanfare: "Fanfare" },
  lookLabel: "Look",
  levelsHint: "Overrides the celebration level for this run (memory only).",
  levels: { full: "Full", calm: "Calm", numbersOnly: "Numbers only" },
  levelAuto: "Use the setting",
  reduced: "Simulate reduced motion",
  intro: "Show intro card again",
};

/** Beta collection (ADR-024) on the Developer page: ambience samples and the sample month. English only. */
export const DEV_COLLECTION = {
  ambienceLabel: "Ambience (beta) · sample",
  ambienceHint: (secs: number) =>
    `Plays one layer alone for ${secs} s or until "Stop ambience", whatever its lock state and the beta switch; never writes the config. Moderate volume.`,
  /** English layer names (the product names live in the locale files). */
  layerNames: {
    rain: "Rain",
    wind: "Wind",
    thunder: "Thunder",
    cafe: "Café",
    fireplace: "Fireplace",
    forest: "Forest & crickets",
    ocean: "Ocean",
    spookyNight: "Spooky night",
    winterCottage: "Winter cottage",
  } satisfies Record<AmbienceLayer, string>,
  stop: "Stop ambience",
  oneShotsLabel: "Spooky night · one-shots",
  oneShots: { owl: "Owl", creak: "Creak", thunder: "Distant thunder" } satisfies Record<SpookyOneShot, string>,
  sampleName: "ambience",
  monthLabel: "Collection · sample month",
  monthHint:
    "Shows Settings → Wygląd → Kolekcja as in that month (level 4, best streak 5: Underwater unlocked, the open seasonal window at 2/3, the others with their windows). A view only: nothing unlocks.",
  monthPick: "Month",
  monthShow: "Show the collection",
  months: [
    "January",
    "February",
    "March",
    "April",
    "May",
    "June",
    "July",
    "August",
    "September",
    "October",
    "November",
    "December",
  ],
  monthSent: (month: string, year: number) => `collection sample: ${month} ${year}`,
};

/** "Presentation mode" (Session card): demo timings + the showcase (`$lib/state/presentation.svelte`), English only. */
export const DEV_PRESENTATION = {
  label: "Presentation mode — demo timings + showcase data",
  hint: (focusSecs: number, breakSecs: number) =>
    `For a live demo: ${focusSecs} s focus / ${breakSecs} s break, an 8-hour workday in Podsumowanie, three weeks of history, the weekly stats, “Co dalej?” and the beta rewards primed so the first block and break show the Start float, a step, the daily goal, the streak, a level up and a pearl. Also hides the Developer tab (Ctrl+Shift+D brings it back) and opens Skupienie. Memory only: nothing is saved or sent; switching it off (or a restart) brings your real data back.`,
  pending: "The demo timings start with the next Start; the running session keeps its lengths. The showcase data shows now.",
  launched: "Started with --presentation: developer mode is on for this run only.",
  now: (on: boolean) => (on ? "Presentation mode on: the showcase shows." : "Presentation mode off: your real data shows again."),
  nextStart: (on: boolean) =>
    `Presentation mode ${on ? "on" : "off"}: the showcase ${on ? "shows" : "is gone"} now; the timings change at the next Start.`,
  name: "presentation mode",
};

/** "Rewards cheats (dev)" on the Developer page (ADR-019, ADR-024): test the unlockables. English only. */
export const DEV_REWARDS_CHEATS = {
  title: "Rewards cheats (dev)",
  hint: "For testing the unlockables. These change your real rewards progress (rewards.json); “Zacznij od nowa” (Settings → Beta) resets it. In presentation mode they change the showcase data only (memory). Each cheat runs the normal unlock rules, so the usual celebrations follow — during focus they wait for the break or Stop.",
  showcaseNote: "Presentation mode is on: the cheats change the showcase data only (memory).",
  cheats: [
    { cheat: "levelUp", label: "+1 level" },
    { cheat: "levelUp5", label: "+5 levels" },
    { cheat: "addSparks100", label: "+100 sparks" },
    { cheat: "streakUp", label: "+1 streak day" },
    { cheat: "streakUp7", label: "+7 streak days" },
    { cheat: "dailyGoalNow", label: "Daily goal now" },
    { cheat: "unlockAll", label: "Unlock all" },
  ] as const satisfies readonly { cheat: RewardsCheat; label: string }[],
  lockAll: "Lock all unlockables",
  lockAllTitle: "Clears the theme, ambience and sticker unlocks (and seasonal goal days); sparks, level and streak stay. The next cheat or real fact re-earns what your level and streak already cover.",
  off: "Beta rewards are off: turn on Settings → Beta features and the sparks first.",
  status: (s: {
    level: number;
    streak: number;
    best: number;
    themes: number;
    themesMax: number;
    ambiences: number;
    ambiencesMax: number;
    stickers: number;
    stickersMax: number;
  }) =>
    `Level ${s.level} · streak ${s.streak} (best ${s.best}) · themes ${s.themes}/${s.themesMax} · ambience ${s.ambiences}/${s.ambiencesMax} · stickers ${s.stickers}/${s.stickersMax}`,
  failed: (label: string, code: string) =>
    code === "rewardsOff" ? `${label}: the beta rewards are off (rewardsOff).` : `${label} failed (${code}).`,
};
