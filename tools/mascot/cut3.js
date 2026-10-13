// Sprite-sheet cutter for the ESRGAN ×4 sheets: frames from analyze2(2) regions (+ attached small extras),
// cut at ×4 from the upscaled sheet, scaled to a common character size, aligned (feet baseline + head centre).
window.cutSheet = async (opts) => {
  const { upscaled, names, refHeight, appHeight = 420, prefix = 'axo', attachDist = 25, keepMasters = true } = opts;
  const fr = window._frames2; const big = fr.filter((f) => !f.pill && f.w > 60 && f.h > 60);
  const small = fr.filter((f) => !f.pill && !(f.w > 60 && f.h > 60) && f.n > 8);
  const owner = new Map(); // region id → frame index in `big`
  big.forEach((f, i) => owner.set(f.i, i));
  for (const s of small) {
    let best = -1, bd = 1e9; const cx = s.x + s.w / 2, cy = s.y + s.h / 2;
    big.forEach((f, i) => { const dx = Math.max(f.x - cx, 0, cx - (f.x + f.w)), dy = Math.max(f.y - cy, 0, cy - (f.y + f.h)); const d = Math.hypot(dx, dy); if (d < bd) { bd = d; best = i; } });
    if (best >= 0 && bd <= attachDist) owner.set(s.i, best);
  }
  if (names.length !== big.length) throw new Error(`expected ${names.length} frames, found ${big.length}`);
  const up = new Image(); up.src = upscaled; await up.decode();
  const uc = document.createElement('canvas'); uc.width = up.naturalWidth; uc.height = up.naturalHeight;
  const uctx = uc.getContext('2d', { willReadFrequently: true }); uctx.drawImage(up, 0, 0);
  const S = up.naturalWidth / W;
  const frames = [];
  for (let fi = 0; fi < big.length; fi++) {
    const regs = new Set([...owner.entries()].filter(([, v]) => v === fi).map(([k]) => k));
    let x0 = 1e9, y0 = 1e9, x1 = -1, y1 = -1;
    for (const [rid] of owner) if (regs.has(rid)) { const f = fr.find((q) => q.i === rid); x0 = Math.min(x0, f.x); y0 = Math.min(y0, f.y); x1 = Math.max(x1, f.x + f.w - 1); y1 = Math.max(y1, f.y + f.h - 1); }
    x0 -= 3; y0 -= 3; x1 += 3; y1 += 3; x0 = Math.max(0, x0); y0 = Math.max(0, y0); x1 = Math.min(W - 1, x1); y1 = Math.min(H - 1, y1);
    const w1 = x1 - x0 + 1, h1 = y1 - y0 + 1;
    // 1× membership, dilated by 2 px so the anti-aliased rim of the upscale is kept
    const m = new Uint8Array(w1 * h1);
    for (let j = 0; j < h1; j++) for (let i = 0; i < w1; i++) { const k = (y0 + j) * W + x0 + i; if (regs.has(window._rlab[k])) m[j * w1 + i] = 1; }
    const md = new Uint8Array(w1 * h1);
    for (let j = 0; j < h1; j++) for (let i = 0; i < w1; i++) { let v = 0; for (let dy = -2; dy <= 2 && !v; dy++) for (let dx = -2; dx <= 2; dx++) { const ii = i + dx, jj = j + dy; if (ii >= 0 && jj >= 0 && ii < w1 && jj < h1 && m[jj * w1 + ii]) { v = 1; break; } } md[j * w1 + i] = v; }
    const w = Math.round(w1 * S), h = Math.round(h1 * S);
    const src = uctx.getImageData(Math.round(x0 * S), Math.round(y0 * S), w, h);
    for (let J = 0; J < h; J++) for (let I = 0; I < w; I++) { const i = Math.min(w1 - 1, (I / S) | 0), j = Math.min(h1 - 1, (J / S) | 0); if (!md[j * w1 + i]) src.data[(J * w + I) * 4 + 3] = 0; }
    // tighten the upscaler's soft alpha: haze below alphaLow disappears, the body becomes fully opaque
    const lo = opts.alphaLow ?? 40, hi = opts.alphaHigh ?? 215;
    for (let p = 3; p < src.data.length; p += 4) { const v = src.data[p]; src.data[p] = v <= lo ? 0 : v >= hi ? 255 : Math.round(((v - lo) / (hi - lo)) * 255); }
    // opaque bounds + anchor (feet = lowest opaque row; x = centroid of the top 45 % = head + headphones)
    let top = h, bottom = 0, left = w, right = 0;
    for (let J = 0; J < h; J++) for (let I = 0; I < w; I++) if (src.data[(J * w + I) * 4 + 3] > 128) { if (J < top) top = J; if (J > bottom) bottom = J; if (I < left) left = I; if (I > right) right = I; }
    let sx = 0, n = 0; const lim = top + (bottom - top) * 0.45;
    for (let J = top; J < lim; J++) for (let I = 0; I < w; I++) if (src.data[(J * w + I) * 4 + 3] > 128) { sx += I; n++; }
    frames.push({ name: names[fi], w, h, img: src, ax: sx / n, ay: bottom, charH: bottom - top + 1 });
  }
  // scale every frame so its character height matches the reference (median of the frames named in refHeight)
  const refs = frames.filter((f) => refHeight.includes(f.name)).map((f) => f.charH).sort((a, b) => a - b);
  const target = refs[(refs.length / 2) | 0];
  const groupScale = (f) => { const peers = frames.filter((q) => q.name.split('-')[0] === f.name.split('-')[0] && q.name.split('-')[0] !== 'happy').map((q) => q.charH).sort((a, b) => a - b); const med = peers.length ? peers[(peers.length / 2) | 0] : f.charH; return target / med; };
  // frames with extras above the head (sparkles) use the median scale of the other non-reference frames
  const nonRef = frames.filter((f) => !refHeight.includes(f.name) && !(opts.extrasGroups ?? ['happy']).includes(f.name.split('-')[0])).map(groupScale).sort((x, y) => x - y);
  const sharedK = nonRef.length ? nonRef[(nonRef.length / 2) | 0] : 1;
  const scaled = frames.map((f) => {
    const k = refHeight.includes(f.name) ? 1 : (opts.extrasGroups ?? ['happy']).includes(f.name.split('-')[0]) ? sharedK : groupScale(f);
    const t = document.createElement('canvas'); t.width = f.w; t.height = f.h; t.getContext('2d').putImageData(f.img, 0, 0);
    const o = document.createElement('canvas'); o.width = Math.round(f.w * k); o.height = Math.round(f.h * k);
    const c = o.getContext('2d'); c.imageSmoothingEnabled = true; c.imageSmoothingQuality = 'high'; c.drawImage(t, 0, 0, o.width, o.height);
    return { ...f, canvas: o, ax: f.ax * k, ay: f.ay * k, k };
  });
  const L = Math.ceil(Math.max(...scaled.map((f) => f.ax))) + 8, R = Math.ceil(Math.max(...scaled.map((f) => f.canvas.width - f.ax))) + 8;
  const U = Math.ceil(Math.max(...scaled.map((f) => f.ay))) + 8, D = Math.ceil(Math.max(...scaled.map((f) => f.canvas.height - f.ay))) + 8;
  const FW = L + R, FH = U + D; const appScale = appHeight / FH; const AW = Math.round(FW * appScale), AH = appHeight;
  const cols = 6, rows = Math.ceil(scaled.length / cols);
  const atlas = document.createElement('canvas'); atlas.width = AW * cols; atlas.height = AH * rows; const actx = atlas.getContext('2d');
  actx.imageSmoothingEnabled = true; actx.imageSmoothingQuality = 'high';
  const manifest = { frameWidth: AW, frameHeight: AH, columns: cols, anchorX: Math.round(L * appScale), anchorY: Math.round(U * appScale), frames: {} };
  for (let i = 0; i < scaled.length; i++) {
    const f = scaled[i]; const full = document.createElement('canvas'); full.width = FW; full.height = FH;
    full.getContext('2d').drawImage(f.canvas, Math.round(L - f.ax), Math.round(U - f.ay));
    if (keepMasters) await post(`${prefix}-master-${f.name}.png`, await toBlob(full));
    const col = i % cols, row = (i / cols) | 0; actx.drawImage(full, col * AW, row * AH, AW, AH);
    manifest.frames[f.name] = { x: col * AW, y: row * AH, scale: +f.k.toFixed(3) };
  }
  const webp = await new Promise((r) => atlas.toBlob(r, 'image/webp', 0.92));
  await post(`${prefix}-atlas.webp`, webp); await post(`${prefix}-atlas.png`, await toBlob(atlas));
  await post(`${prefix}-manifest.json`, new Blob([JSON.stringify(manifest, null, 1)]));
  // dark-background preview of the atlas
  const pv = document.createElement('canvas'); pv.width = atlas.width; pv.height = atlas.height; const pc = pv.getContext('2d'); pc.fillStyle = '#2a2140'; pc.fillRect(0, 0, pv.width, pv.height); pc.drawImage(atlas, 0, 0);
  pc.fillStyle = '#e8dcff'; pc.font = '600 18px system-ui'; pc.textAlign = 'center'; scaled.forEach((f, i) => pc.fillText(f.name, (i % cols) * AW + AW / 2, ((i / cols) | 0) * AH + AH - 8));
  await post(`${prefix}-preview.png`, await toBlob(pv));
  return { frames: scaled.length, master: [FW, FH], app: [AW, AH], atlas: [atlas.width, atlas.height], webpBytes: webp.size, scales: scaled.map((f) => f.name + '×' + f.k.toFixed(2)) };
};
