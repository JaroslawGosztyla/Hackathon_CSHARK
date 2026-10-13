// Ocean waves: every 7–11 s a swell (brown noise, its low-pass opening 350 → 1.6 kHz as it rises) and the wash of the
// breaking wave just after (pink noise above 700 Hz), each wave a little to one side. Between waves a quiet sea.
import { between } from "../rng";
import { kRate, type Kit } from "../kit";

const BASE = 0.32;
/** Level trim (measured offline: wave crests peak ≈ −12 dBFS at volume 1). */
const TRIM_DB = -8.5;

export function ocean(kit: Kit): void {
  const { buffers: b, rng } = kit;
  const out = kit.trim(TRIM_DB);
  const bodyCut = kit.filter("lowpass", 380, 0.6);
  const body = kit.gain(BASE);
  const bodyPan = kit.pan(0);
  kit.chain(kit.loop(b.brown), bodyCut, body, bodyPan, out);

  const washCut = kit.filter("lowpass", 1200, 0.5);
  const wash = kit.gain(BASE * 0.25);
  const washPan = kit.pan(0);
  kit.chain(kit.loop(b.pink), kit.filter("highpass", 700, 0.5), washCut, wash, washPan, out);

  const now = kit.ctx.currentTime;
  for (const param of [body.gain, wash.gain, bodyCut.frequency, washCut.frequency, bodyPan.pan, washPan.pan]) {
    // Swells last seconds: block-rate automation is smooth and cheap.
    kRate(param);
    param.setValueAtTime(param.value, now);
  }

  kit.every({ min: 7, max: 11, first: 0.5 }, (at) => {
    const rise = between(rng, 2.5, 4);
    const fall = between(rng, 3, 4.5);
    const crest = at + rise;
    const peak = between(rng, 0.8, 1.15);
    const side = between(rng, -0.4, 0.4);
    body.gain.linearRampToValueAtTime(BASE * 1.05, at);
    body.gain.linearRampToValueAtTime(peak, crest);
    body.gain.linearRampToValueAtTime(BASE, crest + fall);
    bodyCut.frequency.linearRampToValueAtTime(380, at);
    bodyCut.frequency.linearRampToValueAtTime(between(rng, 1300, 1800), crest);
    bodyCut.frequency.linearRampToValueAtTime(380, crest + fall);
    bodyPan.pan.linearRampToValueAtTime(side, crest);
    // The wash follows the crest and lingers.
    wash.gain.linearRampToValueAtTime(BASE * 0.25, crest - 0.4);
    wash.gain.linearRampToValueAtTime(peak * 0.75, crest + 0.6);
    wash.gain.linearRampToValueAtTime(BASE * 0.25, crest + fall);
    washCut.frequency.linearRampToValueAtTime(1200, crest - 0.4);
    washCut.frequency.linearRampToValueAtTime(5200, crest + 0.6);
    washCut.frequency.linearRampToValueAtTime(1200, crest + fall);
    washPan.pan.linearRampToValueAtTime(-side * 0.5, crest + 0.6);
  });
}
