// Forest and crickets: leaves in a light breeze (pink noise 1–4 kHz, slow swells) and two or three crickets, each a
// soft sine at 4.2–4.8 kHz chirping in short trains (3–4 pulses of 30 ms every 0.7–1.1 s), panned apart.
import { between } from "../rng";
import type { Kit } from "../kit";

const PULSE_S = 0.03;
const PULSE_GAP_S = 0.022;

export function forest(kit: Kit): void {
  const { buffers: b, rng } = kit;
  const leaves = kit.gain(0.5);
  kit.chain(kit.loop(b.pink), kit.filter("bandpass", 2200, 0.55), leaves, kit.bus);
  kit.lfo(0.05, 0.25, leaves.gain);

  const count = 2 + Math.floor(rng() * 2);
  const crickets = Array.from({ length: count }, (_, i) => {
    const gate = kit.gain(0);
    const level = between(rng, 0.016, 0.026);
    kit.chain(
      kit.osc("sine", between(rng, 4200, 4800)),
      gate,
      kit.pan((i - (count - 1) / 2) * 0.55 + between(rng, -0.1, 0.1)),
      kit.bus,
    );
    return { gate, level, offset: between(rng, 0, 0.45), pulses: 3 + Math.floor(rng() * 2) };
  });

  kit.every({ min: 0.7, max: 1.1, first: 0.3 }, (at) => {
    for (const c of crickets) {
      if (rng() < 0.15) continue; // a cricket pauses now and then
      let t = at + c.offset + between(rng, -0.04, 0.04);
      for (let p = 0; p < c.pulses; p++) {
        c.gate.gain.setValueAtTime(0, t);
        c.gate.gain.linearRampToValueAtTime(c.level, t + 0.006);
        c.gate.gain.setValueAtTime(c.level, t + PULSE_S - 0.006);
        c.gate.gain.linearRampToValueAtTime(0, t + PULSE_S);
        t += PULSE_S + PULSE_GAP_S;
      }
    }
  });
}
