// Detects regions in the ×4 sheet: opaque mask (alpha > ALPHA) → dilate by GAP (separable max) → 4-connected labels.
import { decodePng } from './png.mjs';
import { writeFileSync } from 'node:fs';

const [, , src, gapArg = '8', alphaArg = '40'] = process.argv;
const GAP = +gapArg, ALPHA = +alphaArg;
const t0 = Date.now();
const { width: W, height: H, data, hadAlpha } = decodePng(src);
const hist = new Uint32Array(256); for (let k = 3; k < data.length; k += 4) hist[data[k]]++;
console.log(`decoded ${W}x${H} alpha=${hadAlpha} in ${Date.now() - t0} ms; alpha 0:${hist[0]} 255:${hist[255]} between:${W * H - hist[0] - hist[255]}`);

const N = W * H; const ink = new Uint8Array(N);
for (let k = 0; k < N; k++) ink[k] = data[k * 4 + 3] > ALPHA ? 1 : 0;
const tmp = new Uint8Array(N), dil = new Uint8Array(N);
for (let y = 0; y < H; y++) { const r = y * W; let last = -1e9; for (let x = 0; x < W; x++) { if (ink[r + x]) last = x; if (x - last <= GAP) tmp[r + x] = 1; } last = 1e9; for (let x = W - 1; x >= 0; x--) { if (ink[r + x]) last = x; if (last - x <= GAP) tmp[r + x] = 1; } }
for (let x = 0; x < W; x++) { let last = -1e9; for (let y = 0; y < H; y++) { if (tmp[y * W + x]) last = y; if (y - last <= GAP) dil[y * W + x] = 1; } last = 1e9; for (let y = H - 1; y >= 0; y--) { if (tmp[y * W + x]) last = y; if (last - y <= GAP) dil[y * W + x] = 1; } }
const lab = new Int32Array(N).fill(-1); const regions = []; const st = new Int32Array(N);
for (let k = 0; k < N; k++) {
  if (!dil[k] || lab[k] !== -1) continue;
  const id = regions.length; const r = { id, x0: 1e9, y0: 1e9, x1: -1, y1: -1, n: 0, sr: 0, sg: 0, sb: 0, sx: 0, sy: 0 };
  let sp = 0; lab[k] = id; st[sp++] = k;
  while (sp) {
    const j = st[--sp]; const x = j % W, y = (j / W) | 0;
    if (ink[j]) { r.n++; if (x < r.x0) r.x0 = x; if (x > r.x1) r.x1 = x; if (y < r.y0) r.y0 = y; if (y > r.y1) r.y1 = y; r.sr += data[j * 4]; r.sg += data[j * 4 + 1]; r.sb += data[j * 4 + 2]; r.sx += x; r.sy += y; }
    if (x > 0 && dil[j - 1] && lab[j - 1] === -1) { lab[j - 1] = id; st[sp++] = j - 1; }
    if (x < W - 1 && dil[j + 1] && lab[j + 1] === -1) { lab[j + 1] = id; st[sp++] = j + 1; }
    if (y > 0 && dil[j - W] && lab[j - W] === -1) { lab[j - W] = id; st[sp++] = j - W; }
    if (y < H - 1 && dil[j + W] && lab[j + W] === -1) { lab[j + W] = id; st[sp++] = j + W; }
  }
  regions.push(r);
}
const out = regions.filter((r) => r.n > 0).map((r) => ({ id: r.id, x: r.x0, y: r.y0, w: r.x1 - r.x0 + 1, h: r.y1 - r.y0 + 1, n: r.n,
  cx: Math.round(r.sx / r.n), cy: Math.round(r.sy / r.n), rgb: [Math.round(r.sr / r.n), Math.round(r.sg / r.n), Math.round(r.sb / r.n)] }))
  .sort((a, b) => b.n - a.n);
console.log(`${out.length} regions (gap ${GAP}, alpha>${ALPHA}) in ${Date.now() - t0} ms`);
for (const r of out) console.log(`#${r.id} box ${r.x},${r.y} ${r.w}x${r.h} n=${r.n} c=${r.cx},${r.cy} cell r${Math.floor(r.cy / (H / 3)) + 1}c${Math.floor(r.cx / (W / 4)) + 1} rgb=${r.rgb}`);
writeFileSync(src.replace(/\.png$/, `-regions-g${GAP}.json`), JSON.stringify(out));
