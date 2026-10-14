// Linear-time frame detection: opaque mask (minus label pills) → dilate by `gap` (separable max) → label regions.
window.analyze2 = async (gap = 8, alphaMin = 24) => {
  await load();
  const ink = new Uint8Array(W * H);
  for (let k = 0; k < W * H; k++) ink[k] = data[k * 4 + 3] > alphaMin ? 1 : 0;
  // remove label pills: opaque lavender rounded rects with dark text (low saturation lavender, wide & short regions)
  const tmp = new Uint8Array(W * H), dil = new Uint8Array(W * H);
  for (let y = 0; y < H; y++) { let run = -1e9; for (let x = 0; x < W; x++) { if (ink[y * W + x]) run = x; tmp[y * W + x] = x - run <= gap ? 1 : 0; } run = 1e9; for (let x = W - 1; x >= 0; x--) { if (ink[y * W + x]) run = x; if (run - x <= gap) tmp[y * W + x] = 1; } }
  for (let x = 0; x < W; x++) { let run = -1e9; for (let y = 0; y < H; y++) { if (tmp[y * W + x]) run = y; dil[y * W + x] = y - run <= gap ? 1 : 0; } run = 1e9; for (let y = H - 1; y >= 0; y--) { if (tmp[y * W + x]) run = y; if (run - y <= gap) dil[y * W + x] = 1; } }
  const lab = new Int32Array(W * H).fill(-1); const regions = []; const st = [];
  for (let k = 0; k < W * H; k++) {
    if (!dil[k] || lab[k] !== -1) continue;
    const id = regions.length; const r = { x0: 1e9, y0: 1e9, x1: -1, y1: -1, n: 0, lav: 0 }; lab[k] = id; st.push(k);
    while (st.length) { const j = st.pop(); const x = j % W, y = (j / W) | 0;
      if (ink[j]) { r.n++; if (x < r.x0) r.x0 = x; if (x > r.x1) r.x1 = x; if (y < r.y0) r.y0 = y; if (y > r.y1) r.y1 = y; const R = data[j * 4], G = data[j * 4 + 1], B = data[j * 4 + 2]; if (B > 200 && R > 150 && R < 215 && G > 150 && G < 215) r.lav++; }
      const nb = [x > 0 ? j - 1 : -1, x < W - 1 ? j + 1 : -1, y > 0 ? j - W : -1, y < H - 1 ? j + W : -1];
      for (const q of nb) { if (q < 0 || !dil[q] || lab[q] !== -1) continue; lab[q] = id; st.push(q); } }
    regions.push(r);
  }
  window._rlab = lab; window._ink = ink;
  const frames = regions.map((r, i) => ({ i, x: r.x0, y: r.y0, w: r.x1 - r.x0 + 1, h: r.y1 - r.y0 + 1, n: r.n, pill: r.lav / Math.max(1, r.n) > 0.35 }))
    .filter((r) => r.n > 0).sort((a, b) => (Math.abs(a.y - b.y) < 40 ? a.x - b.x : a.y - b.y));
  window._frames2 = frames;
  return frames.map((f) => `${f.i}:${f.x},${f.y},${f.w}x${f.h} n=${f.n}${f.pill ? ' PILL' : ''}`);
};
