// Halloween art shared by the scene and the picker tile: the project's own flat silhouettes (hand-written SVG paths).

/** A bat seen from below, wings spread (viewBox 0 0 100 50; the shoulders sit on y = 20). Symmetric, so it needs no flip. */
export const BAT_WINGS =
  "M53 19C62 10 78 4 97 8Q92 14 88 22Q82 19 76 25Q70 21 64 30Q59 25 54 27Z" +
  "M47 19C38 10 22 4 3 8Q8 14 12 22Q18 19 24 25Q30 21 36 30Q41 25 46 27Z";
export const BAT_BODY =
  "M50 10.8L47.6 11.4 46.2 6.6 45.4 13.5C44.4 15.2 44 18 44.6 21 45.4 26 47.2 30.2 50 32.2 52.8 30.2 54.6 26 55.4 21 56 18 55.6 15.2 54.6 13.5L53.8 6.6 52.4 11.4Z";

/** Pumpkin ribs, back to front (viewBox 0 0 120 100): [cx, cy, rx, ry, shade 0 = darkest]. */
export const PUMPKIN_RIBS: readonly (readonly [number, number, number, number, number])[] = [
  [33, 62, 25, 30, 0],
  [87, 62, 25, 30, 0],
  [47, 61, 23, 34, 1],
  [73, 61, 23, 34, 1],
  [60, 60, 19, 36, 2],
];
export const PUMPKIN_STEM = "M56.5 27.5C55.6 19 58.4 11.8 65.6 7.4l3.6 3.4c-5 3.2-6.6 8.6-5.6 17.4z";
export const PUMPKIN_LEAF = "M64 13c6-5 14-5 19-1-5 1-8 4-11 7-2-3-5-5-8-6z";

/** Carved faces (same viewBox): a friendly set, never scary. */
export const FACES = {
  /** Triangle eyes, a small nose and a grin with two teeth. */
  classic: {
    cut:
      "M35 53l15 0-7.5-13zM70 53l15 0-7.5-13zM56 63h8l-4-6z" +
      "M33 67c9 8 45 8 54 0-3 13-14 21-27 21S36 80 33 67z",
    teeth: "M51 70.5h7v6h-7zM62 79.5h7v7h-7z",
  },
  /** Round eyes and a soft smile. */
  cute: {
    cut:
      "M37.5 50a6.5 6.5 0 1 0 13 0 6.5 6.5 0 1 0-13 0zM69.5 50a6.5 6.5 0 1 0 13 0 6.5 6.5 0 1 0-13 0z" +
      "M42 69c8 10 28 10 36 0-2 9-9 14-18 14s-16-5-18-14z",
    teeth: "",
  },
  /** Happy crescent eyes and a wide open smile. */
  happy: {
    cut: "M35 52c3-9 13-9 16 0-4-4-12-4-16 0zM69 52c3-9 13-9 16 0-4-4-12-4-16 0zM36 64h48c-2 14-12 22-24 22s-22-8-24-22z",
    teeth: "M44 64h7v5h-7zM69 64h7v5h-7z",
  },
} as const;

export type Face = keyof typeof FACES;
