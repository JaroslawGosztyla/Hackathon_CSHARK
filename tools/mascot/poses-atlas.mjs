// Builds the poses atlas for the app: static/mascot/poses-atlas.webp + poses-manifest.json from the owner's 11 pose
// PNGs in art/mascot/poses (×4, RGBA, with poses.json). The poses are only scaled (one common scale for all of them,
// so their relative sizes stay as drawn) and placed on a shared feet line; nothing is redrawn, recoloured, outlined
// or mirrored, and the alpha stays as in the PNGs. Nothing under art/ is written.
//
//   node tools/mascot/poses-atlas.mjs [--scale <s>] [--quality <q>] [--preview <file.png>] [--edge <msedge path>]
//
// Node measures the PNGs (png.mjs); a headless Edge page (poses-atlas.html, served read-only on 127.0.0.1 by this
// script) measures the main atlas, draws the poses with canvas `drawImage` (smoothing "high") and encodes the WebP
// (`toBlob("image/webp")`). Pixel readback happens only in headless Edge (an embedded browser pane adds noise).
//
// Scale: body height of the main idle frame (`blink-1`, from axo-atlas.webp) / body height of `05-look-up`, where body
// height = feet line − first row with alpha >= 128; `--scale` overrides it after a visual check.
// Anchor: the feet line (poses.json `baselineY`) and the body's mean x (pixels with alpha >= 128 in the lower half of
// the figure). The cell keeps every pixel with alpha > 0 plus a 4 px margin; the manifest's anchor is chosen so that
// a pose drawn with its anchor on the main atlas's anchor stands where the main frame's body stands.
// Same inputs → same manifest (the scale is rounded, every number is computed from the files).
import { decodePng } from './png.mjs';
import { spawn } from 'node:child_process';
import { createServer } from 'node:http';
import { existsSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const HERE = dirname(fileURLToPath(import.meta.url));
const REPO = resolve(HERE, '..', '..');
const POSES_DIR = join(REPO, 'art', 'mascot', 'poses');
const MAIN_DIR = join(REPO, 'static', 'mascot');
const OUT_ATLAS = join(MAIN_DIR, 'poses-atlas.webp');
const OUT_MANIFEST = join(MAIN_DIR, 'poses-manifest.json');

/** Alpha from which a pixel counts as the figure (feet line, body height, body centre). */
const ALPHA_INK = 128;
/** Transparent margin around the union of all poses in a cell (atlas pixels, each side). */
const PAD = 4;
const COLUMNS = 4;
/** The reference frames of the scale. */
const MAIN_FRAME = 'blink-1';
const REFERENCE_POSE = '05-look-up';
const SCALE_DECIMALS = 4;
/** WebP quality; over budget → the fallback quality (never a smaller scale). */
const QUALITY = 0.9;
const FALLBACK_QUALITY = 0.85;
const BUDGET_BYTES = 600 * 1000;
const EDGE_START_MS = 20_000;
const EDGE_CANDIDATES = [
  'C:\\Program Files (x86)\\Microsoft\\Edge\\Application\\msedge.exe',
  'C:\\Program Files\\Microsoft\\Edge\\Application\\msedge.exe',
  '/Applications/Microsoft Edge.app/Contents/MacOS/Microsoft Edge',
  '/usr/bin/microsoft-edge',
];

function parseArgs(argv) {
  const opts = { scale: null, quality: null, preview: null, edge: process.env.EDGE_PATH ?? null };
  for (let i = 0; i < argv.length; i++) {
    const key = argv[i], value = argv[i + 1];
    if (key === '--scale') { opts.scale = Number(value); i++; }
    else if (key === '--quality') { opts.quality = Number(value); i++; }
    else if (key === '--preview') { opts.preview = resolve(value); i++; }
    else if (key === '--edge') { opts.edge = value; i++; }
    else throw new Error(`unknown argument ${key}`);
  }
  if (opts.scale !== null && !(opts.scale > 0 && opts.scale <= 1)) throw new Error('--scale must be in (0, 1]');
  if (opts.quality !== null && !(opts.quality > 0 && opts.quality <= 1)) throw new Error('--quality must be in (0, 1]');
  return opts;
}

/** Feet line, body height, body centre and the extents of every visible pixel, in the PNG's own pixels. */
function measurePose(pose) {
  const { width: W, height: H, data } = decodePng(join(POSES_DIR, pose.file));
  if (W !== pose.width || H !== pose.height) throw new Error(`${pose.file}: ${W}x${H}, poses.json says ${pose.width}x${pose.height}`);
  const alpha = (x, y) => data[(y * W + x) * 4 + 3];
  let inkTop = -1, minX = W, maxX = -1, minY = H, maxY = -1;
  for (let y = 0; y < H; y++) for (let x = 0; x < W; x++) {
    const a = alpha(x, y);
    if (a > 0) { if (x < minX) minX = x; if (x > maxX) maxX = x; if (y < minY) minY = y; if (y > maxY) maxY = y; }
    if (a >= ALPHA_INK && inkTop < 0) inkTop = y;
  }
  const base = pose.baselineY;
  if (inkTop < 0 || !(base > inkTop && base < H)) throw new Error(`${pose.file}: no figure above baselineY ${base}`);
  const from = Math.ceil(base - 0.5 * (base - inkTop));
  let sum = 0, n = 0;
  for (let y = from; y <= base; y++) for (let x = 0; x < W; x++) if (alpha(x, y) >= ALPHA_INK) { sum += x; n++; }
  const cx = Math.round(sum / n);
  // Continuous coordinates of the anchor: the centre of the body's mean column and of the feet row.
  const ax = cx + 0.5, ay = base + 0.5;
  return {
    name: pose.name, file: pose.file, width: W, height: H, baselineY: base, inkTop, cx,
    bodyHeight: base - inkTop,
    left: ax - minX, right: maxX + 1 - ax, up: ay - minY, down: maxY + 1 - ay,
  };
}

function contentType(path) {
  if (path.endsWith('.html')) return 'text/html; charset=utf-8';
  if (path.endsWith('.png')) return 'image/png';
  if (path.endsWith('.webp')) return 'image/webp';
  return 'application/json';
}

/** Read-only server for the page: the tool page, the pose PNGs listed in poses.json and the main atlas. */
function serve(poseFiles) {
  const routes = new Map([
    ['/poses-atlas.html', join(HERE, 'poses-atlas.html')],
    ['/main/axo-manifest.json', join(MAIN_DIR, 'axo-manifest.json')],
    ['/main/axo-atlas.webp', join(MAIN_DIR, 'axo-atlas.webp')],
    ...poseFiles.map((file) => [`/poses/${file}`, join(POSES_DIR, file)]),
  ]);
  const server = createServer((req, res) => {
    const path = new URL(req.url ?? '/', 'http://127.0.0.1').pathname;
    const file = routes.get(path);
    if (req.method !== 'GET' || !file) { res.writeHead(404).end(); return; }
    try {
      res.writeHead(200, { 'content-type': contentType(file), 'cache-control': 'no-store' }).end(readFileSync(file));
    } catch (e) {
      res.writeHead(500).end(String(e));
    }
  });
  return new Promise((ok) => server.listen(0, '127.0.0.1', () => ok(server)));
}

const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

async function waitFor(what, fn, ms = EDGE_START_MS) {
  const until = Date.now() + ms;
  for (;;) {
    try { const v = await fn(); if (v) return v; } catch { /* not yet */ }
    if (Date.now() > until) throw new Error(`timed out waiting for ${what}`);
    await sleep(100);
  }
}

class Cdp {
  #ws; #seq = 0; #calls = new Map();
  constructor(ws) {
    this.#ws = ws;
    ws.onmessage = (e) => {
      const msg = JSON.parse(String(e.data));
      const call = this.#calls.get(msg.id);
      if (!call) return;
      this.#calls.delete(msg.id);
      if (msg.error) call.reject(new Error(msg.error.message)); else call.resolve(msg.result);
    };
  }
  static async connect(url) {
    const ws = new WebSocket(url);
    await new Promise((ok, fail) => { ws.onopen = ok; ws.onerror = () => fail(new Error(`cannot connect to ${url}`)); });
    return new Cdp(ws);
  }
  send(method, params = {}) {
    const id = ++this.#seq;
    return new Promise((resolve, reject) => {
      this.#calls.set(id, { resolve, reject });
      this.#ws.send(JSON.stringify({ id, method, params }));
    });
  }
  /** Evaluates `expression` in the page (promises awaited) and returns its JSON value. */
  async eval(expression) {
    const r = await this.send('Runtime.evaluate', { expression, awaitPromise: true, returnByValue: true });
    if (r.exceptionDetails) throw new Error(r.exceptionDetails.exception?.description ?? r.exceptionDetails.text);
    return r.result.value;
  }
  close() { this.#ws.close(); }
}

async function withEdge(edgePath, pageUrl, run) {
  const profile = mkdtempSync(join(tmpdir(), 'focusmode-poses-'));
  const edge = spawn(edgePath, [
    '--headless=new', '--disable-gpu', '--no-first-run', '--no-default-browser-check', '--disable-extensions',
    '--disable-background-networking', '--remote-debugging-port=0', `--user-data-dir=${profile}`, pageUrl,
  ], { stdio: 'ignore' });
  const exited = new Promise((r) => edge.once('exit', r));
  let browser = null;
  try {
    const portFile = join(profile, 'DevToolsActivePort');
    const port = await waitFor('Edge DevTools', () => existsSync(portFile) && readFileSync(portFile, 'utf8').split(/\r?\n/)[0]);
    const base = `http://127.0.0.1:${port}`;
    browser = await Cdp.connect((await (await fetch(`${base}/json/version`)).json()).webSocketDebuggerUrl);
    const target = await waitFor('the tool page', async () =>
      (await (await fetch(`${base}/json/list`)).json()).find((t) => t.type === 'page' && t.url.startsWith(pageUrl)));
    const page = await Cdp.connect(target.webSocketDebuggerUrl);
    try {
      await waitFor('the page script', () => page.eval('window.ready === true'));
      return await run(page);
    } finally {
      page.close();
    }
  } finally {
    if (browser) { await browser.send('Browser.close').catch(() => {}); browser.close(); }
    await Promise.race([exited, sleep(5000)]);
    if (edge.exitCode === null) edge.kill();
    rmSync(profile, { recursive: true, force: true, maxRetries: 10, retryDelay: 200 });
  }
}

const round = (v, decimals) => Number(v.toFixed(decimals));

async function main() {
  const opts = parseArgs(process.argv.slice(2));
  const edgePath = opts.edge ?? EDGE_CANDIDATES.find((p) => existsSync(p));
  if (!edgePath) throw new Error('Microsoft Edge not found: pass --edge <path> (or EDGE_PATH)');

  const meta = JSON.parse(readFileSync(join(POSES_DIR, 'poses.json'), 'utf8'));
  const poses = [...meta.poses].sort((a, b) => a.name.localeCompare(b.name)).map(measurePose);
  const reference = poses.find((p) => p.name === REFERENCE_POSE);
  if (!reference) throw new Error(`reference pose ${REFERENCE_POSE} missing`);

  const server = await serve(poses.map((p) => p.file));
  const origin = `http://127.0.0.1:${server.address().port}`;
  try {
    await withEdge(edgePath, `${origin}/poses-atlas.html`, async (page) => {
      const mainRef = await page.eval(`window.measureMain(${JSON.stringify({
        manifestUrl: '/main/axo-manifest.json', atlasUrl: '/main/axo-atlas.webp', frame: MAIN_FRAME,
      })})`);
      const mainBody = mainRef.anchorY - mainRef.top;
      const measured = round(mainBody / reference.bodyHeight, SCALE_DECIMALS);
      const s = opts.scale ?? measured;

      // Cell: every visible pixel of every pose around the body anchor, plus the margin.
      const half = Math.max(...poses.map((p) => Math.max(p.left, p.right)));
      const up = Math.max(...poses.map((p) => p.up));
      const down = Math.max(...poses.map((p) => p.down));
      const frameWidth = 2 * Math.ceil(half * s) + 2 * PAD;
      const bodyX = frameWidth / 2;
      const anchorY = PAD + Math.ceil(up * s);
      const frameHeight = anchorY + Math.ceil(down * s) + PAD;
      // The main frames' anchor is the head's column; their body stands at `bodyCx`. Shifting the poses' anchor by the
      // same amount puts a pose's body where the main body is when both anchors coincide.
      const bodyOffset = Math.round(mainRef.anchorX - mainRef.bodyCx);
      const anchorX = bodyX + bodyOffset;
      const rows = Math.ceil(poses.length / COLUMNS);
      const width = COLUMNS * frameWidth, height = rows * frameHeight;

      const frames = {};
      const cells = poses.map((p, i) => {
        const x = (i % COLUMNS) * frameWidth, y = Math.floor(i / COLUMNS) * frameHeight;
        frames[p.name] = { x, y, scale: 1 };
        return {
          url: `/poses/${p.file}`, srcWidth: p.width, srcHeight: p.height, x, y, w: frameWidth, h: frameHeight,
          dx: x + bodyX - (p.cx + 0.5) * s, dy: y + anchorY + 0.5 - (p.baselineY + 0.5) * s, dw: p.width * s, dh: p.height * s,
        };
      });

      console.log(`main ${MAIN_FRAME}: top ${mainRef.top}, feet ${mainRef.bottom} (anchorY ${mainRef.anchorY}), body ${mainBody}, ` +
        `body x ${mainRef.bodyCx.toFixed(1)} (anchorX ${mainRef.anchorX}, offset ${bodyOffset})`);
      console.log(`${REFERENCE_POSE}: body ${reference.bodyHeight} → scale ${measured}${opts.scale !== null ? ` (using --scale ${s})` : ''}`);
      for (const p of poses) {
        console.log(`  ${p.name.padEnd(24)} ${p.width}x${p.height} feet ${p.baselineY} body x ${p.cx} body ${p.bodyHeight}` +
          ` → ${Math.round(p.bodyHeight * s)} px`);
      }
      console.log(`cell ${frameWidth}x${frameHeight}, anchor ${anchorX},${anchorY}; atlas ${width}x${height} (${COLUMNS}x${rows})`);

      let quality = opts.quality ?? QUALITY;
      let out = await page.eval(`window.buildAtlas(${JSON.stringify({ width, height, cells, quality })})`);
      if (out.type !== 'image/webp') throw new Error(`the browser encoded ${out.type}, not WebP`);
      if (out.size > BUDGET_BYTES && opts.quality === null) {
        console.log(`${out.size} bytes at quality ${quality} is over budget; retrying at ${FALLBACK_QUALITY}`);
        quality = FALLBACK_QUALITY;
        out = await page.eval(`window.buildAtlas(${JSON.stringify({ width, height, cells, quality })})`);
      }
      if (out.size > BUDGET_BYTES) throw new Error(`atlas is ${out.size} bytes, over the ${BUDGET_BYTES} byte budget`);
      if (out.edgeAlpha !== 0) throw new Error(`a pose touches its cell edge (alpha ${out.edgeAlpha}): the cell is too small`);

      const manifest = {
        frameWidth, frameHeight, columns: COLUMNS, anchorX, anchorY, poseScale: s, webpQuality: quality,
        source: 'art/mascot/poses', reference: { main: MAIN_FRAME, pose: REFERENCE_POSE }, frames,
      };
      writeFileSync(OUT_ATLAS, Buffer.from(out.base64, 'base64'));
      writeFileSync(OUT_MANIFEST, `${JSON.stringify(manifest, null, 1)}\n`);
      console.log(`wrote ${OUT_ATLAS} (${out.size} bytes, quality ${quality}) and ${OUT_MANIFEST}`);
      console.log(`FRAME_GEOMETRY.poses in src/lib/mascot/assets.svelte.ts must read: ` +
        `{ frameWidth: ${frameWidth}, frameHeight: ${frameHeight}, anchorX: ${anchorX}, anchorY: ${anchorY} }`);

      if (opts.preview) {
        const preview = await page.eval(`window.buildPreview(${JSON.stringify({
          main: mainRef, mainAtlasUrl: '/main/axo-atlas.webp',
          mainFrame: JSON.parse(readFileSync(join(MAIN_DIR, 'axo-manifest.json'), 'utf8')).frames[MAIN_FRAME],
          poses: Object.values(frames), cellWidth: frameWidth, cellHeight: frameHeight, anchorX, anchorY,
        })})`);
        writeFileSync(opts.preview, Buffer.from(preview.base64, 'base64'));
        console.log(`wrote preview ${opts.preview}`);
      }
    });
  } finally {
    server.close();
  }
}

main().catch((e) => {
  console.error(e instanceof Error ? e.message : e);
  process.exit(1);
});
