// Kept for compatibility: the UI texts live in `$lib/i18n` (one file per language, Polish source in `pl.ts`).
// `strings` always reads the language in use, so existing `strings.x.y` usages switch with it.
export { errorText, GAME_LAUNCHERS, LANGUAGES, launcherList, strings } from "./i18n/index.svelte";
