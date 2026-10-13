// Beta collection themes (ADR-024): their ids, colour schemes, the theme on screen and the tokens of every collection
// theme. Each collection theme lives in its own folder `src/lib/themes/<id>/`, where <id> is the `RewardTheme` value
// (also the `data-theme` value):
//   tokens.css      `:root[data-theme="<id>"], [data-theme="<id>"]:not(:root) { … }`: the full token set of a base theme
//                   in app.css (incl. --app-bg, the reward tokens and --quiet-mark). The doubled selector (specificity 0,2,0)
//                   wins over app.css's shared `:root, [data-theme]` blocks whatever order the CSS loads in (the popup
//                   loads this module before app.css). Loaded here, so every window that applies themes (the main window
//                   and the phase popup) has it.
//   Scene.svelte    the window background, drawn inside Background.svelte (registry in `./scenes.ts`).
//   Preview.svelte  the art of the theme's card in the picker (registry in `./scenes.ts`).
// Folders are independent: adding one never touches a shared file. Without the beta none of this is listed or applied.
import type { Config, RewardTheme, Theme, UnlocksView } from "$lib/types";

export type ColorScheme = "dark" | "light";

/** A base theme (free for everyone) or a collection theme (beta). */
export type ThemeId = Theme | RewardTheme;

/**
 * Every collection theme in catalogue order (focus-core `RewardTheme::ALL`), with the colour scheme of its form
 * controls and scrollbars (`color-scheme` in its tokens.css must match).
 */
const REWARD_SCHEMES: Record<RewardTheme, ColorScheme> = {
  underwater: "dark",
  rainyWindow: "dark",
  forestCabin: "dark",
  sakuraNight: "dark",
  newYear: "dark",
  valentines: "dark",
  easter: "light",
  summer: "dark",
  halloween: "dark",
  christmas: "dark",
};

export const REWARD_THEMES = Object.keys(REWARD_SCHEMES) as RewardTheme[];

// The tokens of every collection theme (CSS side effect: Vite adds each file to the page).
import.meta.glob("./*/tokens.css", { eager: true });

export function isRewardTheme(value: unknown): value is RewardTheme {
  return typeof value === "string" && (REWARD_THEMES as readonly string[]).includes(value);
}

export function rewardColorScheme(theme: RewardTheme): ColorScheme {
  return REWARD_SCHEMES[theme];
}

/** `theme` is unlocked in `unlocks` (`RewardsStatus.unlocks`). */
export function isUnlocked(unlocks: UnlocksView, theme: RewardTheme): boolean {
  return unlocks.themes.some((item) => item.theme === theme && item.state === "unlocked");
}

/**
 * The theme on screen (ADR-024): the collection theme (`Config.rewardTheme`) while the beta is on and the theme is
 * unlocked, else the base theme (`Config.theme`, which always stays the user's last base theme). `unlocks` null = not
 * known yet (no status so far, or an older shell): the config is trusted, since the shell never saves a collection
 * theme that is not unlocked and clears it on "Zacznij od nowa".
 */
export function effectiveTheme(
  cfg: Pick<Config, "theme" | "rewardTheme" | "betaFeatures">,
  unlocks: UnlocksView | null | undefined,
): ThemeId {
  const chosen = cfg.rewardTheme;
  if (!cfg.betaFeatures || !isRewardTheme(chosen)) return cfg.theme;
  if (unlocks && !isUnlocked(unlocks, chosen)) return cfg.theme;
  return chosen;
}
