// Cuts the 11 poses from the ×4 sheet: regions as in poses-analyze.mjs (alpha > 40, dilate 8 px at ×4), extras attached by grid cell,
// pixels masked by region membership (never by rectangle), soft alpha haze cleaned (<= 40 -> 0, >= 215 -> 255, in between unchanged), uniform 24 px margin.
import { decodePng, encodePng } from './png.mjs';
import { writeFileSync, mkdirSync } from 'node:fs';
import { join } from 'node:path';

const [, , src, outDir, GAP = '8', ALPHA = '40'] = process.argv;
const MARGIN = 24, LO = 40, HI = 215, SCALE = 4;
const POSES = {
  'r1c1': '01-point-excited', 'r1c2': '02-clasped-hands-heart', 'r1c3': '03-thinking', 'r1c4': '04-lying-awake',
  'r2c2': '05-look-up', 'r2c3': '06-amazed-sparkles', 'r2c4': '07-sleeping',
  'r3c1': '08-back-view', 'r3c2': '09-lying-look-up', 'r3c3': '10-holding-heart', 'r3c4': '11-point-happy',
};
const { width: W, height: H, data } = decodePng(src);
const N = W * H; const gap = +GAP, alpha = +ALPHA;

// --- regions (same as poses-analyze.mjs) ---
const ink = new Uint8Array(N); for (let k = 0; k < N; k++) ink[k] = data[k * 4 + 3] > alpha ? 1 : 0;
const tmp = new Uint8Array(N), dil = new Uint8Array(N);
for (let y = 0; y < H; y++) { const r = y * W; let last = -1e9; for (let x = 0; x < W; x++) { if (ink[r + x]) last = x; if (x - last <= gap) tmp[r + x] = 1; } last = 1e9; for (let x = W - 1; x >= 0; x--) { if (ink[r + x]) last = x; if (last - x <= gap) tmp[r + x] = 1; } }
for (let x = 0; x < W; x++) { let last = -1e9; for (let y = 0; y < H; y++) { if (tmp[y * W + x]) last = y; if (y - last <= gap) dil[y * W + x] = 1; } last = 1e9; for (let y = H - 1; y >= 0; y--) { if (tmp[y * W + x]) last = y; if (last - y <= gap) dil[y * W + x] = 1; } }
const lab = new Int32Array(N).fill(-1); const regions = []; const st = new Int32Array(N);
for (let k = 0; k < N; k++) {
  if (!dil[k] || lab[k] !== -1) continue;
  const id = regions.length; const r = { id, x0: 1e9, y0: 1e9, x1: -1, y1: -1, n: 0, sx: 0, sy: 0 };
  let sp = 0; lab[k] = id; st[sp++] = k;
  while (sp) {
    const j = st[--sp]; const x = j % W, y = (j / W) | 0;
    if (ink[j]) { r.n++; r.sx += x; r.sy += y; if (x < r.x0) r.x0 = x; if (x > r.x1) r.x1 = x; if (y < r.y0) r.y0 = y; if (y > r.y1) r.y1 = y; }
    if (x > 0 && dil[j - 1] && lab[j - 1] === -1) { lab[j - 1] = id; st[sp++] = j - 1; }
    if (x < W - 1 && dil[j + 1] && lab[j + 1] === -1) { lab[j + 1] = id; st[sp++] = j + 1; }
    if (y > 0 && dil[j - W] && lab[j - W] === -1) { lab[j - W] = id; st[sp++] = j - W; }
    if (y < H - 1 && dil[j + W] && lab[j + W] === -1) { lab[j + W] = id; st[sp++] = j + W; }
  }
  regions.push(r);
}
const live = regions.filter((r) => r.n > 0);
const cellOf = (r) => `r${Math.floor(r.sy / r.n / (H / 3)) + 1}c${Math.floor(r.sx / r.n / (W / 4)) + 1}`;
const bodies = live.filter((r) => r.n > 100000);
if (bodies.length !== 11) throw new Error(`expected 11 poses, found ${bodies.length}`);
const byCell = new Map();
for (const b of bodies) { const c = cellOf(b); if (!POSES[c] || byCell.has(c)) throw new Error(`unexpected body in ${c}`); byCell.set(c, { body: b, parts: [b] }); }
// extras: same grid cell as the pose; if a cell had several poses we would take the nearest, here each cell holds one
const report = [];
for (const r of live.filter((q) => q.n <= 100000)) {
  const c = cellOf(r); const owner = byCell.get(c);
  const cx = r.sx / r.n, cy = r.sy / r.n;
  let near = null, nd = 1e18;
  for (const [cc, o] of byCell) { const b = o.body; const dx = Math.max(b.x0 - cx, 0, cx - b.x1), dy = Math.max(b.y0 - cy, 0, cy - b.y1); const d = Math.hypot(dx, dy); if (d < nd) { nd = d; near = cc; } }
  if (!owner) throw new Error(`extra #${r.id} in empty cell ${c}`);
  if (near !== c) console.log(`WARN extra #${r.id} nearest body is ${near} but sits in cell ${c}`);
  owner.parts.push(r);
  report.push(`#${r.id} ${r.x1 - r.x0 + 1}x${r.y1 - r.y0 + 1}@${r.x0},${r.y0} n=${r.n} -> ${POSES[c]} (dist to body ${Math.round(nd)} px)`);
}
console.log(report.join('\n'));

mkdirSync(outDir, { recursive: true });
const meta = { source: 'axolotl-poses-sheet.webp', scale: SCALE, model: 'realesrgan-x4plus-anime', margin: MARGIN, alphaClean: { zeroAtOrBelow: LO, opaqueAtOrAbove: HI, between: 'unchanged' }, poses: [] };
const boxes = [];
// soft haze only: alpha <= 40 -> 0, alpha >= 215 -> 255, values in between are kept unchanged (no stretch)
const clean = (v) => (v <= LO ? 0 : v >= HI ? 255 : v);
for (const cell of Object.keys(POSES)) {
  const { body, parts } = byCell.get(cell); const name = POSES[cell];
  const ids = new Set(parts.map((p) => p.id));
  // bounds of member pixels that survive cleaning
  let x0 = 1e9, y0 = 1e9, x1 = -1, y1 = -1;
  for (const p of parts) for (let y = p.y0; y <= p.y1; y++) for (let x = p.x0; x <= p.x1; x++) { const k = y * W + x; if (lab[k] === p.id && data[k * 4 + 3] > LO) { if (x < x0) x0 = x; if (x > x1) x1 = x; if (y < y0) y0 = y; if (y > y1) y1 = y; } }
  const bx = x0 - MARGIN, by = y0 - MARGIN, bw = x1 - x0 + 1 + 2 * MARGIN, bh = y1 - y0 + 1 + 2 * MARGIN;
  const img = new Uint8Array(bw * bh * 4); let foreign = 0, baselineY = -1;
  for (let j = 0; j < bh; j++) for (let i = 0; i < bw; i++) {
    const X = bx + i, Y = by + j; if (X < 0 || Y < 0 || X >= W || Y >= H) continue;
    const k = Y * W + X; const a = data[k * 4 + 3];
    if (!ids.has(lab[k])) { if (lab[k] !== -1 && a > LO) foreign++; continue; }
    const A = clean(a); if (!A) continue;
    const o = (j * bw + i) * 4; img[o] = data[k * 4]; img[o + 1] = data[k * 4 + 1]; img[o + 2] = data[k * 4 + 2]; img[o + 3] = A;
    if (lab[k] === body.id && A >= 128 && j > baselineY) baselineY = j;
  }
  const file = `${name}.png`; const bytes = encodePng(join(outDir, file), bw, bh, img);
  meta.poses.push({ file, name, row: +cell[1], col: +cell[3], x: bx, y: by, width: bw, height: bh, baselineY });
  boxes.push({ name, bx, by, bw, bh });
  console.log(`${file} ${bw}x${bh} at ${bx},${by} baselineY=${baselineY} parts=${parts.length} maskedForeign=${foreign} bytes=${bytes}`);
}
for (let a = 0; a < boxes.length; a++) for (let b = a + 1; b < boxes.length; b++) {
  const p = boxes[a], q = boxes[b];
  const ox = Math.min(p.bx + p.bw, q.bx + q.bw) - Math.max(p.bx, q.bx), oy = Math.min(p.by + p.bh, q.by + q.bh) - Math.max(p.by, q.by);
  if (ox > 0 && oy > 0) console.log(`OVERLAP ${p.name} / ${q.name}: ${ox}x${oy}`);
}
writeFileSync(join(outDir, 'poses.json'), JSON.stringify(meta, null, 2) + '\n');
