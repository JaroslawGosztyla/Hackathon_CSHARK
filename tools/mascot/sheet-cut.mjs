// Cuts every pose from a ×4 RGBA sheet laid out in rows (any number per row, gaps allowed), e.g. the output of
// white-to-alpha.mjs. Regions: alpha > INK, dilated by GAP px and labelled; regions above BODY_MIN ink px are poses,
// smaller ones (marks such as "!", "?", zzZ, hearts, sparkles, motion lines) join the pose whose box is nearest.
// Pixels are taken by region membership (never by rectangle), colours and alpha exactly as in the sheet, with a
// uniform MARGIN. Rows: poses sorted by centre y, a new row when the next centre is more than ROW_GAP of the height
// lower; the grid column comes from the centre x over the widest row's column count.
// Usage: node tools/mascot/sheet-cut.mjs <sheet-x4-rgba.png> <out-dir> <source-name> <name1,name2,...>
// Names are given in reading order (rows top to bottom, left to right). Writes <name>.png per pose and poses.json.
import { decodePng, encodePng } from './png.mjs';
import { writeFileSync, mkdirSync } from 'node:fs';
import { join } from 'node:path';

const [, , src, outDir, source, nameList] = process.argv;
if (!nameList) throw new Error('usage: node sheet-cut.mjs <sheet-x4-rgba.png> <out-dir> <source-name> <name1,name2,...>');
const NAMES = nameList.split(',');
const INK = 40, GAP = 8, BODY_MIN = 200000, MARGIN = 24, ROW_GAP = 0.1, SCALE = 4;

const { width: W, height: H, data, hadAlpha } = decodePng(src);
if (!hadAlpha) throw new Error('the sheet has no alpha channel: run white-to-alpha.mjs first');
const N = W * H;
const ink = new Uint8Array(N); for (let k = 0; k < N; k++) ink[k] = data[k * 4 + 3] > INK ? 1 : 0;
const tmp = new Uint8Array(N), dil = new Uint8Array(N);
for (let y = 0; y < H; y++) { const r = y * W; let last = -1e9; for (let x = 0; x < W; x++) { if (ink[r + x]) last = x; if (x - last <= GAP) tmp[r + x] = 1; } last = 1e9; for (let x = W - 1; x >= 0; x--) { if (ink[r + x]) last = x; if (last - x <= GAP) tmp[r + x] = 1; } }
for (let x = 0; x < W; x++) { let last = -1e9; for (let y = 0; y < H; y++) { if (tmp[y * W + x]) last = y; if (y - last <= GAP) dil[y * W + x] = 1; } last = 1e9; for (let y = H - 1; y >= 0; y--) { if (tmp[y * W + x]) last = y; if (last - y <= GAP) dil[y * W + x] = 1; } }
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
const bodies = live.filter((r) => r.n > BODY_MIN).map((b) => ({ body: b, parts: [b], cx: b.sx / b.n, cy: b.sy / b.n }));
if (bodies.length !== NAMES.length) throw new Error(`found ${bodies.length} poses, got ${NAMES.length} names`);

// rows and reading order
bodies.sort((a, b) => a.cy - b.cy);
const rows = [];
for (const b of bodies) { const row = rows.at(-1); if (row && b.cy - row.at(-1).cy <= ROW_GAP * H) row.push(b); else rows.push([b]); }
const columns = Math.max(...rows.map((r) => r.length));
const ordered = [];
rows.forEach((row, i) => { row.sort((a, b) => a.cx - b.cx); for (const b of row) { b.row = i + 1; b.col = Math.floor(b.cx / (W / columns)) + 1; ordered.push(b); } });
ordered.forEach((b, i) => (b.name = NAMES[i]));

// marks join the nearest pose (distance from the mark's centre to the pose's box)
const boxDist = (b, x, y) => Math.hypot(Math.max(b.x0 - x, 0, x - b.x1), Math.max(b.y0 - y, 0, y - b.y1));
for (const r of live.filter((q) => q.n <= BODY_MIN)) {
  const cx = r.sx / r.n, cy = r.sy / r.n;
  const near = ordered.map((o) => ({ o, d: boxDist(o.body, cx, cy) })).sort((a, b) => a.d - b.d);
  near[0].o.parts.push(r);
  const close = near[1] && near[1].d < 1.5 * near[0].d + 40 ? ` (CHECK: ${near[1].o.name} at ${Math.round(near[1].d)} px)` : '';
  console.log(`mark ${r.x1 - r.x0 + 1}x${r.y1 - r.y0 + 1}@${r.x0},${r.y0} ink=${r.n} -> ${near[0].o.name} (${Math.round(near[0].d)} px)${close}`);
}

mkdirSync(outDir, { recursive: true });
const meta = { source, scale: SCALE, model: 'realesrgan-x4plus-anime', margin: MARGIN, background: 'white-to-alpha.mjs', poses: [] };
const boxes = [];
for (const o of ordered) {
  const ids = new Set(o.parts.map((p) => p.id));
  let x0 = 1e9, y0 = 1e9, x1 = -1, y1 = -1;
  for (const p of o.parts) for (let y = p.y0 - GAP; y <= p.y1 + GAP; y++) for (let x = p.x0 - GAP; x <= p.x1 + GAP; x++) {
    if (x < 0 || y < 0 || x >= W || y >= H) continue;
    const k = y * W + x; if (lab[k] === p.id && data[k * 4 + 3] > 0) { if (x < x0) x0 = x; if (x > x1) x1 = x; if (y < y0) y0 = y; if (y > y1) y1 = y; }
  }
  const bx = x0 - MARGIN, by = y0 - MARGIN, bw = x1 - x0 + 1 + 2 * MARGIN, bh = y1 - y0 + 1 + 2 * MARGIN;
  const img = new Uint8Array(bw * bh * 4); let foreign = 0, baselineY = -1;
  for (let j = 0; j < bh; j++) for (let i = 0; i < bw; i++) {
    const X = bx + i, Y = by + j; if (X < 0 || Y < 0 || X >= W || Y >= H) continue;
    const k = Y * W + X; const a = data[k * 4 + 3]; if (!a) continue;
    if (!ids.has(lab[k])) { if (lab[k] !== -1) foreign++; continue; }
    const q = (j * bw + i) * 4; img[q] = data[k * 4]; img[q + 1] = data[k * 4 + 1]; img[q + 2] = data[k * 4 + 2]; img[q + 3] = a;
    if (lab[k] === o.body.id && a >= 128 && j > baselineY) baselineY = j;
  }
  const file = `${o.name}.png`; const bytes = encodePng(join(outDir, file), bw, bh, img);
  meta.poses.push({ file, name: o.name, row: o.row, col: o.col, x: bx, y: by, width: bw, height: bh, baselineY });
  boxes.push({ name: o.name, bx, by, bw, bh });
  console.log(`${file} r${o.row}c${o.col} ${bw}x${bh} at ${bx},${by} baselineY=${baselineY} parts=${o.parts.length} maskedForeign=${foreign} bytes=${bytes}`);
}
for (let a = 0; a < boxes.length; a++) for (let b = a + 1; b < boxes.length; b++) {
  const p = boxes[a], q = boxes[b];
  const ox = Math.min(p.bx + p.bw, q.bx + q.bw) - Math.max(p.bx, q.bx), oy = Math.min(p.by + p.bh, q.by + q.bh) - Math.max(p.by, q.by);
  if (ox > 0 && oy > 0) console.log(`OVERLAP ${p.name} / ${q.name}: ${ox}x${oy}`);
}
writeFileSync(join(outDir, 'poses.json'), JSON.stringify(meta, null, 2) + '\n');
