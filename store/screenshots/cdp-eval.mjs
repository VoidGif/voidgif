// cdp-eval.mjs — evaluate a JS expression in the VoidGif WebView2 page over the
// Chrome DevTools Protocol. Used by capture-cdp.ps1 to switch theme / open the
// export dialog for screenshots WITHOUT injecting OS mouse/keyboard input.
//
//   node cdp-eval.mjs <debugPort> <base64-encoded-js>
//
// The expression is base64-encoded to avoid shell/quoting issues.
// Requires Node 22+ (global fetch + global WebSocket). Tested on Node 24.

const port = process.argv[2] ?? "9223";
const expr = Buffer.from(process.argv[3] ?? "MQ==", "base64").toString("utf8");

const res = await fetch(`http://127.0.0.1:${port}/json`);
const targets = await res.json();
const page = targets.find((t) => t.type === "page" && t.webSocketDebuggerUrl);
if (!page) {
  console.error("no page target on port " + port);
  process.exit(2);
}

const ws = new WebSocket(page.webSocketDebuggerUrl);
await new Promise((resolve, reject) => {
  ws.onopen = resolve;
  ws.onerror = (e) => reject(new Error("ws error"));
});

let id = 0;
function send(method, params = {}) {
  return new Promise((resolve) => {
    const mid = ++id;
    const onMsg = (ev) => {
      const m = JSON.parse(ev.data);
      if (m.id === mid) {
        ws.removeEventListener("message", onMsg);
        resolve(m);
      }
    };
    ws.addEventListener("message", onMsg);
    ws.send(JSON.stringify({ id: mid, method, params }));
  });
}

await send("Runtime.enable");
const r = await send("Runtime.evaluate", {
  expression: expr,
  awaitPromise: true,
  returnByValue: true,
});
console.log(JSON.stringify(r.result ?? r.error ?? {}));
ws.close();
process.exit(0);
