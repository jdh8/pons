#!/usr/bin/env python3
"""Cross-check the Rust exact-mass counter against the web Partner counter.

Pin (iv) of docs/exact-posterior.md Phase 0.  Serves web/ with a harness
appended to app.js; a real browser recounts each fixture with companionCount
and posts the counts back, and this script compares them with the Rust counts.

    cargo run --release --example probe-exact-mass -- --fixtures -c 400 \\
        | grep '^{' > fixtures.jsonl
    scripts/web-counter-crosscheck.py fixtures.jsonl [PKG_DIR] &
    firefox --headless http://127.0.0.1:8139/      # or open it in any browser

PKG_DIR defaults to web/pkg, which must be built from the same source as
web/app.js (web/README.md).  A clean exit is a pass.
"""
import http.server
import json
import math
import pathlib
import sys
import threading

PORT = 8139
WEB = pathlib.Path(__file__).resolve().parent.parent / 'web'
PKG = pathlib.Path(sys.argv[2]) if len(sys.argv) > 2 else WEB / 'pkg'
fixtures = [json.loads(line) for line in open(sys.argv[1])]

# Appended to app.js, so it runs inside the module and calls its own functions.
# My hand is a box naming every honor; a reading's boxes are partner's query,
# their points on the `up` gauge — the bidder's scale.
HARNESS = r"""
;(async () => {
  try {
    while (!JOINT) await new Promise((r) => setTimeout(r, 100));
    const fixtures = await (await fetch('/fixtures.json')).json();
    const mineVars = handVars(MINE, 1);
    const partnerVars = { ...handVars(0, 1), ...handVars(MINE, 1, 'my.') };
    const text = ([lo, hi]) => `${lo}-${hi}`;
    const out = [];
    for (const f of fixtures) {
      const mine = [makeBox(parseCells([...splitHand(f.hand), '', ''], mineVars), mineVars.p)];
      const query = f.boxes.map(([c, d, h, s, p]) =>
        makeBox(parseCells([text(s), text(h), text(d), text(c), text(p), ''], partnerVars), partnerVars.p));
      const n = query.length <= MAX_BOXES && companionCount(mine, [ANY_BOX], query);
      out.push(n ? { mine: n.mine, known: n.known, query: n.query } : null);
    }
    await fetch('/result', { method: 'POST', body: JSON.stringify({ out, browser: navigator.userAgent }) });
  } catch (e) {
    await fetch('/result', { method: 'POST', body: JSON.stringify({ error: `${e}\n${e.stack}` }) });
  }
})();
"""

TYPES = {'.js': 'text/javascript', '.wasm': 'application/wasm', '.html': 'text/html',
         '.css': 'text/css', '.json': 'application/json'}
result = {}


class Handler(http.server.BaseHTTPRequestHandler):
    def log_message(self, *args):
        pass

    def do_GET(self):
        path = self.path.split('?')[0]
        if path == '/fixtures.json':
            body = json.dumps(fixtures).encode()
        elif path == '/app.js':
            body = (WEB / 'app.js').read_bytes() + HARNESS.encode()
        else:
            name = path.lstrip('/') or 'index.html'
            file = PKG / name[4:] if name.startswith('pkg/') else WEB / name
            if '..' in name or not file.is_file():
                self.send_error(404)
                return
            body = file.read_bytes()
        self.send_response(200)
        self.send_header('Content-Type', TYPES.get(pathlib.Path(path).suffix, 'text/html'))
        self.send_header('Content-Length', str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    def do_POST(self):
        result.update(json.loads(self.rfile.read(int(self.headers['Content-Length']))))
        self.send_response(204)
        self.end_headers()
        threading.Thread(target=self.server.shutdown).start()


print(f'open http://127.0.0.1:{PORT}/ in a browser', file=sys.stderr)
http.server.ThreadingHTTPServer(('127.0.0.1', PORT), Handler).serve_forever()

if 'error' in result:
    sys.exit(f"browser error: {result['error']}")
TOTAL = math.comb(39, 13)
exact = near = skipped = 0
for fixture, got in zip(fixtures, result['out']):
    if got is None:  # past MAX_BOXES or MAX_CELLS: the web counter declined
        skipped += 1
        continue
    # The web names my spots by count, so it counts my hand `mine` times over.
    mine = round(got['mine'])
    for have, want in ((got['known'], mine * TOTAL), (got['query'], mine * fixture['count'])):
        if want < 2 ** 53:
            assert have == want, (fixture, got, want)
            exact += 1
        else:  # past a double's integers: the web counter's own tolerance
            assert abs(have - want) <= 1e-9 * want, (fixture, got, want)
            near += 1
print(result['browser'])
print(f'{len(fixtures)} fixtures: {exact} counts equal exactly, '
      f'{near} within 1e-9 (beyond 2^53), {skipped} fixtures declined')
