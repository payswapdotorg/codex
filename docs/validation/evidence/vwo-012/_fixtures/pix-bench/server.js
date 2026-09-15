#!/usr/bin/env node
'use strict';
/**
 * PhotoPrep — purpose-built GUI photo tool fixture (VWO-012, §5 note 4).
 * A plausible small photo-prep product for the NST-D desktop lane:
 *   - filmstrip of seeded site photos, before/after preview
 *   - deterministic image backend = ffmpeg (resize / rotate)
 *   - per-photo visual CONFIRM gate: Save-over-original is SERVER-enforced
 *     (no save without a recorded confirm — the TD-12-style guard + the
 *     task's "confirm each result on screen before saving")
 * Zero dependencies. 127.0.0.1 only. No credentials.
 */
const http = require('http');
const { execFile } = require('child_process');
const fs = require('fs');
const path = require('path');

const PORT = parseInt(process.argv[2] || '4203', 10);
const WS = process.env.NSTD_WS || '/tmp/nstd-workspace';
const PHOTOS_DIR = path.join(WS, 'site-photos');
const TMP_DIR = path.join(WS, '.pix-tmp');
fs.mkdirSync(TMP_DIR, { recursive: true });
const TEMPLATE = '800x600';

function list() {
  return fs.readdirSync(PHOTOS_DIR).filter((f) => f.endsWith('.png')).sort().map((f) => ({
    name: f.replace(/\.png$/, ''), file: f,
  }));
}
function jws(res, code, obj) {
  const body = JSON.stringify(obj);
  res.writeHead(code, { 'Content-Type': 'application/json', 'Content-Length': Buffer.byteLength(body) });
  res.end(body);
}
function readBody(req) {
  return new Promise((resolve) => {
    let b = ''; req.on('data', (c) => (b += c)); req.on('end', () => { try { resolve(JSON.parse(b || '{}')); } catch { resolve({}); } });
  });
}
function run(cmd, args) {
  return new Promise((resolve, reject) =>
    execFile(cmd, args, { timeout: 30000 }, (err, so, se) => (err ? reject(new Error(se || so || String(err))) : resolve(so))));
}
function safePhoto(p) { return String(p || '').replace(/[^a-zA-Z0-9._-]/g, ''); }

// SERVER-side durable op state (the save gate reads this)
const ops = {};

const PAGE = `<!doctype html><html><head><meta charset="utf-8">
<title>PhotoPrep — Site Photo Tool</title>
<style>
 body{font-family:system-ui,sans-serif;margin:0;background:#14161a;color:#e8e6e1}
 .bar{background:#1f232b;padding:.5rem .9rem;display:flex;gap:1rem;align-items:center}
 h1{font-size:1rem;margin:0;font-weight:600}
 .bar span{font-size:.8rem;color:#9aa3ad}
 main{display:grid;grid-template-columns:170px 1fr;height:calc(100vh - 48px)}
 .strip{background:#181b20;overflow:auto;padding:.5rem}
 .strip button{display:block;width:100%;background:none;border:none;color:#e8e6e1;font:inherit;padding:.5rem;border-radius:6px;cursor:pointer;text-align:left}
 .strip button.sel{background:#2b313c;outline:2px solid #4a90d9;outline-offset:-2px}
 .stage{display:grid;grid-template-rows:1fr auto;padding:1rem;gap:1rem;overflow:auto}
 .views{display:grid;grid-template-columns:1fr 1fr;gap:1rem}
 .view{background:#0d0f12;border:1px solid #2b313c;border-radius:8px;padding:.5rem;text-align:center}
 .view img{max-width:100%;max-height:52vh}
 .view .lbl{font-size:.75rem;color:#9aa3ad;letter-spacing:.06em;text-transform:uppercase;margin-bottom:.4rem}
 .controls{display:flex;gap:.7rem;align-items:center;justify-content:center;flex-wrap:wrap}
 .controls button{background:#2b313c;color:#e8e6e1;border:1px solid #3a424f;font:inherit;padding:.5rem 1rem;border-radius:7px;cursor:pointer}
 .controls button:hover{background:#343c49}
 .controls button:disabled{opacity:.4;cursor:not-allowed}
 #confirmBtn.on{background:#1e6b3a;border-color:#2e8b4f}
 #saveBtn.on{background:#8a5a1e;border-color:#b0762a}
 .statusbar{position:fixed;bottom:0;left:170px;right:0;background:#101317;font-size:.78rem;color:#9aa3ad;padding:.3rem 1rem}
</style></head><body>
<div class="bar"><h1>PhotoPrep</h1><span>report template: ${TEMPLATE} · PNG batch prep</span></div>
<main>
 <nav class="strip" id="strip" aria-label="Photos"></nav>
 <section class="stage">
  <div class="views">
   <div class="view"><div class="lbl">before (original)</div><img id="before" alt="original photo" /></div>
   <div class="view"><div class="lbl">after (result)</div><img id="after" alt="processed photo" /></div>
  </div>
  <div class="controls">
   <button id="resizeBtn">Resize to template (${TEMPLATE})</button>
   <button id="rotateBtn">Rotate 90° CW</button>
   <button id="confirmBtn">Confirm result</button>
   <button id="saveBtn" disabled>Save over original</button>
  </div>
 </section>
</main>
<div class="statusbar" id="status">select a photo</div>
<script>
let cur = null, ops = {};
const status = document.getElementById('status');
async function refreshStrip() {
  const photos = await (await fetch('/api/photos')).json();
  const strip = document.getElementById('strip');
  strip.innerHTML = '';
  for (const p of photos) {
    const b = document.createElement('button');
    b.textContent = p.name + (ops[p.name] ? (ops[p.name].confirmed ? ' ✓✓' : ' ·') : '');
    if (cur === p.name) b.classList.add('sel');
    b.onclick = () => select(p.name);
    strip.appendChild(b);
  }
}
async function select(name) {
  cur = name; status.textContent = 'photo: ' + name;
  document.getElementById('before').src = '/photo/' + name + '.png';
  const st = ops[name];
  document.getElementById('after').src = st ? '/preview/' + st.token + '.png' : '/photo/' + name + '.png';
  refreshStrip(); syncButtons();
}
function syncButtons() {
  const st = ops[cur];
  document.getElementById('confirmBtn').disabled = !st;
  document.getElementById('confirmBtn').classList.toggle('on', !!(st && st.confirmed));
  document.getElementById('saveBtn').disabled = !(st && st.confirmed);
  document.getElementById('saveBtn').classList.toggle('on', !!(st && st.confirmed));
}
async function applyOp(kind) {
  if (!cur) return;
  const d = await (await fetch('/api/op', { method: 'POST', headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ photo: cur, kind, prev: ops[cur] || null }) })).json();
  if (d.error) { status.textContent = d.error; return; }
  ops[cur] = d.state;
  document.getElementById('after').src = '/preview/' + d.state.token + '.png?v=' + Date.now();
  status.textContent = cur + ': ' + d.state.desc + ' — review the result, then Confirm';
  refreshStrip(); syncButtons();
}
document.getElementById('resizeBtn').onclick = () => applyOp('resize');
document.getElementById('rotateBtn').onclick = () => applyOp('rotate');
document.getElementById('confirmBtn').onclick = async () => {
  const d = await (await fetch('/api/confirm', { method: 'POST', headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ photo: cur }) })).json();
  if (d.error) { status.textContent = d.error; return; }
  ops[cur].confirmed = true; status.textContent = cur + ': result confirmed — Save over original is enabled';
  refreshStrip(); syncButtons();
};
document.getElementById('saveBtn').onclick = async () => {
  const d = await (await fetch('/api/save', { method: 'POST', headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ photo: cur }) })).json();
  if (d.error) { status.textContent = d.error; return; }
  status.textContent = cur + ': saved over original (' + d.size + ')';
  delete ops[cur]; select(cur);
};
refreshStrip().then(() => { const first = document.querySelector('.strip button'); if (first) first.click(); });
</script></body></html>`;

http.createServer(async (req, res) => {
  const u = new URL(req.url, 'http://localhost');
  if (req.method === 'GET' && u.pathname === '/') {
    res.writeHead(200, { 'Content-Type': 'text/html; charset=utf-8' });
    return res.end(PAGE);
  }
  if (req.method === 'GET' && u.pathname === '/api/photos') return jws(res, 200, list());
  if (req.method === 'GET' && (u.pathname.startsWith('/photo/') || u.pathname.startsWith('/preview/'))) {
    const isPrev = u.pathname.startsWith('/preview/');
    const base = isPrev ? TMP_DIR : PHOTOS_DIR;
    const f = path.join(base, safePhoto(u.pathname.split('/').pop()));
    if (!fs.existsSync(f)) { res.writeHead(404); return res.end('nf'); }
    res.writeHead(200, { 'Content-Type': 'image/png' });
    return res.end(fs.readFileSync(f));
  }
  if (req.method === 'POST' && u.pathname === '/api/op') {
    const b = await readBody(req);
    const photo = safePhoto((b.photo || '')).replace(/\.png$/, '');
    if (!list().some((p) => p.name === photo)) return jws(res, 200, { error: 'unknown photo' });
    const prev = b.prev && b.prev.token && b.prev.op ? b.prev : null;
    const src = prev ? path.join(TMP_DIR, prev.token + '.png') : path.join(PHOTOS_DIR, photo + '.png');
    const token = photo + '-' + Date.now();
    fs.mkdirSync(TMP_DIR, { recursive: true }); // self-heal (workspace reseeds wipe it)
    const dst = path.join(TMP_DIR, token + '.png');
    const desc = prev ? prev.desc + ' + ' : '';
    try {
      if (b.kind === 'resize') {
        await run('ffmpeg', ['-y', '-hide_banner', '-loglevel', 'error', '-i', src, '-vf', 'scale=' + TEMPLATE, '-frames:v', '1', dst]);
      } else if (b.kind === 'rotate') {
        await run('ffmpeg', ['-y', '-hide_banner', '-loglevel', 'error', '-i', src, '-vf', 'transpose=1', '-frames:v', '1', dst]);
      } else return jws(res, 200, { error: 'unknown op' });
    } catch (e) { return jws(res, 200, { error: 'ffmpeg failed: ' + e.message }); }
    const op = prev ? prev.op + '+' + b.kind : b.kind;
    const st = { token, op, confirmed: false, desc: desc + b.kind, photo };
    ops[photo] = st; // SERVER-side durable op state (the save gate reads this)
    return jws(res, 200, { state: st });
  }
  if (req.method === 'POST' && u.pathname === '/api/confirm') {
    const b = await readBody(req);
    const photo = safePhoto((b.photo || '')).replace(/\.png$/, '');
    const st = ops[photo];
    if (!st) return jws(res, 200, { error: 'nothing to confirm — apply an op first' });
    st.confirmed = true; // durable server-side confirm record (audit gate)
    return jws(res, 200, { ok: true, confirmedAt: new Date().toISOString() });
  }
  if (req.method === 'POST' && u.pathname === '/api/save') {
    const b = await readBody(req);
    const photo = safePhoto((b.photo || '')).replace(/\.png$/, '');
    const st = ops[photo];
    // the confirm gate is SERVER-enforced: no save without a recorded confirm
    if (!st || !st.confirmed) return jws(res, 200, { error: 'confirm the on-screen result before saving' });
    const src = path.join(TMP_DIR, st.token + '.png');
    const dst = path.join(PHOTOS_DIR, photo + '.png');
    fs.copyFileSync(src, dst);
    const size = fs.statSync(dst).size + ' bytes';
    delete ops[photo];
    return jws(res, 200, { ok: true, size });
  }
  jws(res, 404, { error: 'not found' });
}).listen(PORT, '127.0.0.1', () => console.log('PhotoPrep on http://127.0.0.1:' + PORT + '/'));
