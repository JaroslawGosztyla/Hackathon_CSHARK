// Envelope helpers shared by the ambience layers (ADR-024). Every one-shot fades in (no hard attack) and fades out to
// silence before its source stops, so nothing clicks.

/** Silence at `at`, a linear rise to `peak` over `attack`, an exponential fall to −60 dB over `decay`. Returns the end. */
export function swell(param: AudioParam, at: number, attack: number, peak: number, decay: number): number {
  const top = at + Math.max(0.005, attack);
  const end = top + Math.max(0.01, decay);
  param.setValueAtTime(0, at);
  param.linearRampToValueAtTime(peak, top);
  param.exponentialRampToValueAtTime(Math.max(1e-5, peak * 1e-3), end);
  param.linearRampToValueAtTime(0, end + 0.02);
  return end + 0.02;
}

/** A tone glide: `from` at `at` to `to` at `at + seconds` (exponential, like a voice). */
export function glide(param: AudioParam, at: number, from: number, to: number, seconds: number): void {
  param.setValueAtTime(Math.max(1, from), at);
  param.exponentialRampToValueAtTime(Math.max(1, to), at + Math.max(0.01, seconds));
}
