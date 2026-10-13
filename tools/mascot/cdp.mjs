// Evaluates an expression in the headless Edge page on :9333 and prints the JSON result.
const expr = process.argv[2];
const t = (await (await fetch('http://127.0.0.1:9333/json/list')).json()).find((p) => p.type === 'page' && p.url.includes('8777'));
const ws = new WebSocket(t.webSocketDebuggerUrl); await new Promise((r) => (ws.onopen = r));
const res = await new Promise((r) => { ws.onmessage = (e) => { const m = JSON.parse(e.data); if (m.id === 1) r(m.result); }; ws.send(JSON.stringify({ id: 1, method: 'Runtime.evaluate', params: { expression: expr, awaitPromise: true, returnByValue: true } })); });
console.log(JSON.stringify(res.result?.value ?? res.exceptionDetails?.exception?.description ?? res));
process.exit(0);
