// Makes the opaque white background of an owner's sheet transparent (input: the ×4 Real-ESRGAN PNG, RGB or RGBA).
// 1. Background = near-white pixels (every channel >= BG_MIN) connected to the image border. White enclosed by the
//    drawing (eye shines, the pinkish highlight on the head, slivers under a headphone cup) stays part of the art: on
//    the owner's transparent sheets that place is drawn, not a hole.
// 2. Rim: from the background inwards, each anti-aliased edge pixel that is a mix of white and the nearest solid colour
//    (the most saturated pixel within WIN px, usually the dark outline) is un-mixed: alpha = its share of that colour,
//    colour = the colour without the white. The walk stops at the first solid pixel, so it never crosses an outline and
//    never touches the inside of a drawing. Pixels that are not such a mix (residual > RESIDUAL) stay opaque and unchanged.
// Everything else keeps its colour and becomes fully opaque: the art is not recoloured (CLAUDE.md rule 10).
// Usage: node tools/mascot/white-to-alpha.mjs <sheet-x4.png> <out-rgba.png>
import { decodePng, encodePng } from './png.mjs';

const [, , src, out] = process.argv;
if (!src || !out) throw new Error('usage: node white-to-alpha.mjs <sheet-x4.png> <out-rgba.png>');
const BG_MIN = 240, WIN = 6, RIM = 16, RESIDUAL = 24, SOLID = 0.85, DROP = 8;

const { width: W, height: H, data } = decodePng(src);
const N = W * H;
const bgish = new Uint8Array(N);
for (let k = 0; k < N; k++) {
  const o = k * 4;
  bgish[k] = data[o] >= BG_MIN && data[o + 1] >= BG_MIN && data[o + 2] >= BG_MIN ? 1 : 0;
}

// 1. background: flood from the border
const bg = new Uint8Array(N);
const stack = new Int32Array(N);
const flood = (seeds, mark) => {
  let sp = 0;
  for (const s of seeds) if (bgish[s] && !mark[s]) { mark[s] = 1; stack[sp++] = s; }
  while (sp) {
    const j = stack[--sp];
    const x = j % W;
    if (x > 0 && bgish[j - 1] && !mark[j - 1]) { mark[j - 1] = 1; stack[sp++] = j - 1; }
    if (x < W - 1 && bgish[j + 1] && !mark[j + 1]) { mark[j + 1] = 1; stack[sp++] = j + 1; }
    if (j >= W && bgish[j - W] && !mark[j - W]) { mark[j - W] = 1; stack[sp++] = j - W; }
    if (j < N - W && bgish[j + W] && !mark[j + W]) { mark[j + W] = 1; stack[sp++] = j + W; }
  }
};
const border = [];
for (let x = 0; x < W; x++) border.push(x, N - W + x);
for (let y = 0; y < H; y++) border.push(y * W, y * W + W - 1);
flood(border, bg);

// 2. rim un-mixing, walking inwards from the background up to RIM px
const alpha = new Uint8Array(N).fill(255);
for (let k = 0; k < N; k++) if (bg[k]) alpha[k] = 0;
const state = new Uint8Array(N); // 0 untouched, 1 queued
const queue = new Int32Array(N); const depth = new Uint8Array(N);
let qh = 0, qt = 0;
for (let k = 0; k < N; k++) {
  if (bg[k]) continue;
  const x = k % W;
  if ((x > 0 && bg[k - 1]) || (x < W - 1 && bg[k + 1]) || (k >= W && bg[k - W]) || (k < N - W && bg[k + W])) { state[k] = 1; depth[k] = 1; queue[qt++] = k; }
}
let unmixed = 0, solid = 0, foreign = 0;
while (qh < qt) {
  const k = queue[qh++];
  const o = k * 4, r = data[o], g = data[o + 1], b = data[o + 2];
  // the most saturated (farthest from white) non-background pixel nearby
  const x = k % W, y = (k / W) | 0;
  let best = -1, fr = 0, fg = 0, fb = 0;
  for (let yy = Math.max(0, y - WIN); yy <= Math.min(H - 1, y + WIN); yy++) for (let xx = Math.max(0, x - WIN); xx <= Math.min(W - 1, x + WIN); xx++) {
    const j = yy * W + xx; if (bg[j]) continue;
    const q = j * 4, dr = 255 - data[q], dg = 255 - data[q + 1], db = 255 - data[q + 2];
    const d = dr * dr + dg * dg + db * db;
    if (d > best) { best = d; fr = data[q]; fg = data[q + 1]; fb = data[q + 2]; }
  }
  const wr = 255 - r, wg = 255 - g, wb = 255 - b, vr = 255 - fr, vg = 255 - fg, vb = 255 - fb;
  const a = best > 0 ? Math.min(1, Math.max(0, (wr * vr + wg * vg + wb * vb) / best)) : 1;
  const res = Math.hypot(wr - a * vr, wg - a * vg, wb - a * vb);
  if (res > RESIDUAL) { foreign++; continue; } // not a white mix: keep as drawn
  if (a >= SOLID) { solid++; continue; }       // reached the solid colour: stop here
  unmixed++;
  const A = Math.round(a * 255);
  if (A < DROP) { alpha[k] = 0; data[o] = fr; data[o + 1] = fg; data[o + 2] = fb; }
  else {
    alpha[k] = A;
    data[o] = Math.min(255, Math.max(0, Math.round((r - (1 - a) * 255) / a)));
    data[o + 1] = Math.min(255, Math.max(0, Math.round((g - (1 - a) * 255) / a)));
    data[o + 2] = Math.min(255, Math.max(0, Math.round((b - (1 - a) * 255) / a)));
  }
  if (depth[k] >= RIM) continue;
  for (const j of [x > 0 ? k - 1 : -1, x < W - 1 ? k + 1 : -1, k - W, k + W]) {
    if (j < 0 || j >= N || bg[j] || state[j]) continue;
    state[j] = 1; depth[j] = depth[k] + 1; queue[qt++] = j;
  }
}
// background pixels: transparent, colour of their nearest drawing is irrelevant, keep white for a neutral fringe
for (let k = 0; k < N; k++) data[k * 4 + 3] = alpha[k];
const bytes = encodePng(out, W, H, data);
let opaque = 0; for (let k = 0; k < N; k++) if (alpha[k] === 255) opaque++;
console.log(`${W}x${H}: background ${N - opaque - unmixed} px, rim un-mixed ${unmixed}, rim solid ${solid}, rim not a white mix ${foreign}, bytes ${bytes}`);
