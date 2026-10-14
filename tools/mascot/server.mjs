// Tiny local helper: serves this folder and saves POSTed files into ./out (mascot sprite extraction).
import { createServer } from 'node:http';
import { readFile, writeFile, mkdir } from 'node:fs/promises';
import { join, extname, basename } from 'node:path';

const root = new URL('.', import.meta.url).pathname.replace(/^\/([A-Za-z]:)/, '$1');
const types = { '.html': 'text/html', '.js': 'text/javascript', '.webp': 'image/webp', '.png': 'image/png', '.json': 'application/json' };
await mkdir(join(root, 'out'), { recursive: true });

createServer(async (req, res) => {
  try {
    const url = new URL(req.url, 'http://127.0.0.1');
    if (req.method === 'POST' && url.pathname === '/save') {
      const name = basename(url.searchParams.get('name') ?? '');
      if (!/^[a-z0-9._-]+$/i.test(name)) { res.writeHead(400).end('bad name'); return; }
      const chunks = [];
      for await (const c of req) chunks.push(c);
      let body = Buffer.concat(chunks);
      if (url.searchParams.get('b64') === '1') body = Buffer.from(body.toString('utf8'), 'base64');
      await writeFile(join(root, 'out', name), body);
      res.writeHead(200).end('ok');
      return;
    }
    const file = join(root, basename(url.pathname === '/' ? 'tool.html' : url.pathname));
    const data = await readFile(file);
    res.writeHead(200, { 'content-type': types[extname(file)] ?? 'application/octet-stream' }).end(data);
  } catch (e) {
    res.writeHead(404).end(String(e));
  }
}).listen(8777, '127.0.0.1', () => console.log('mascot tool on http://127.0.0.1:8777/'));
