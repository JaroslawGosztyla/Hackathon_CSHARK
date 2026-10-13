// Polish texts of the theme picker (UI texts are Polish; code is English).
import type { Theme } from "$lib/bindings/Theme";
import type { MoonPhaseName } from "./moon";

export const themeStrings = {
  legend: "Motyw aplikacji",
  hint: "Kolory i tło głównego okna oraz okienek faz. Zmiana zapisuje się od razu.",
  selected: "wybrany",

  names: {
    midnight: "Północ",
    vaporwave: "Vaporwave",
    lunar: "Fazy Księżyca",
    autumn: "Przytulna jesień",
    dark: "Ciemny",
    light: "Jasny",
    pixel: "8-bit",
  } satisfies Record<Theme, string>,

  descriptions: {
    midnight: "Ciepła, fioletowa noc z deszczem za oknem. Domyślny.",
    vaporwave: "Róż, cyjan i fiolet: paskowane słońce nad siatką w perspektywie.",
    lunar: "Srebro i indygo pod gwiazdami.",
    autumn: "Brązy i bursztyn, powoli opadające liście.",
    dark: "Neutralna szarość, bez dekoracji w tle.",
    light: "Jasne tło i ciemny tekst o wysokim kontraście, bez dekoracji.",
    pixel: "Kanciaste ramki, twarde cienie i pikselowe gwiazdy.",
  } satisfies Record<Theme, string>,

  /** Added to the "Fazy Księżyca" description: the moon in the background shows today's phase. */
  moonToday: (phase: string, litPct: number) => `Księżyc w tle ma dzisiejszą fazę: ${phase} (${litPct}% tarczy).`,

  moonPhases: {
    new: "nów",
    waxingCrescent: "przybywający sierp",
    firstQuarter: "pierwsza kwadra",
    waxingGibbous: "przybywający garb",
    full: "pełnia",
    waningGibbous: "ubywający garb",
    lastQuarter: "ostatnia kwadra",
    waningCrescent: "ubywający sierp",
  } satisfies Record<MoonPhaseName, string>,
} as const;
