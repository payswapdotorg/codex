#!/usr/bin/env node
'use strict';
/**
 * TeamPad — purpose-built rich-editor fixture (VWO-013 NST-B008, §5 note 4).
 * A plausible team web editor: contenteditable body (bold/lists via
 * execCommand), attachment upload (stored server-side, re-served
 * byte-identically), save + reopen round-trip via the fixture API.
 * Zero dependencies. 127.0.0.1 only. No credentials.
 */
const http = require('http');
const fs = require('fs');
const path = require('path');
const crypto = require('crypto');

const PORT = parseInt(process.argv[2] || '4208', 10);
process.on('unhandledRejection', (e) => console.error('unhandledRejection:', e));
process.on('uncaughtException', (e) => console.error('uncaughtException:', e));
const STORE_DIR = '/tmp/nstd-vwo013-store';
fs.mkdirSync(STORE_DIR, { recursive: true });

const docs = { 'post-mortem': { html: '', attachments: [] } }; // docId -> {html, attachments}

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

const PAGE = `<!doctype html><html><head><meta charset="utf-8">
<title>TeamPad — Post-mortem</title>
<style>
 body{font-family:system-ui,sans-serif;margin:0;background:#fbfbf9;color:#26251f}
 .bar{background:#2e2b24;color:#fff;padding:.4rem .9rem;display:flex;gap:.6rem;align-items:center}
 h1{font-size:.95rem;margin:0 1rem 0 0;font-weight:600}
 .bar button{background:#423d33;color:#fff;border:1px solid #57503f;font:inherit;padding:.3rem .8rem;border-radius:6px;cursor:pointer}
 .bar button:hover{background:#4d473a}
 main{max-width:46rem;margin:1rem auto;padding:0 1rem}
 #editor{min-height:14rem;background:#fff;border:1px solid #d8d4c8;border-radius:10px;padding:1rem 1.2rem;outline:none;font:1rem/1.65 Georgia,serif}
 #editor:focus{border-color:#8b7f5f}
 .att{margin-top:1rem}
 .att h2{font-size:.85rem}
 .att ul{font-size:.85rem;color:#55503f}
 .status{font-size:.78rem;color:#6b6552;margin-top:.6rem}
</style></head><body>
<div class="bar"><h1>TeamPad</h1>
 <button id="b-bold" title="Bold"><b>B</b></button>
 <button id="b-ul" title="Bulleted list">• List</button>
 <button id="b-attach" title="Attach file">Attach…</button>
 <button id="b-save">Save</button>
 <input type="file" id="file" style="display:none">
</div>
<main>
 <div id="editor" contenteditable="true" aria-label="Document body"></div>
 <div class="att"><h2>Attachments</h2><ul id="attList"><li>(none)</li></ul></div>
 <div class="status" id="status">ready — doc: post-mortem</div>
</main>
<script>
const editor = document.getElementById('editor'), status = document.getElementById('status');
function refreshAtts(list) {
  document.getElementById('attList').innerHTML = (list && list.length)
    ? list.map(a => '<li>' + a.name + ' (' + a.size + ' B, sha256 ' + a.sha + ')</li>').join('')
    : '<li>(none)</li>';
}
document.getElementById('b-bold').onclick = () => document.execCommand('bold');
document.getElementById('b-ul').onclick = () => document.execCommand('insertUnorderedList');
document.getElementById('b-attach').onclick = () => document.getElementById('file').click();
document.getElementById('file').addEventListener('change', async function () {
  if (!this.files.length) return;
  const f = this.files[0];
  const buf = await f.arrayBuffer();
  const bytes = Array.from(new Uint8Array(buf));
  const r = await fetch('/api/attach', { method: 'POST', headers: {'Content-Type': 'application/json'},
    body: JSON.stringify({docId: 'post-mortem', name: f.name, bytes}) });
  const d = await r.json();
  if (d.ok) { refreshAtts(d.attachments); status.textContent = 'attached ' + d.name + ' (' + d.size + ' B)'; }
  this.value = '';
});
document.getElementById('b-save').onclick = async () => {
  const r = await fetch('/api/save', {method:'POST', headers:{'Content-Type':'application/json'},
    body: JSON.stringify({docId:'post-mortem', html: editor.innerHTML})});
  const d = await r.json();
  status.textContent = d.ok ? 'saved at ' + new Date().toISOString().slice(11,19) : ('save failed: ' + d.error);
};
(async () => { // reopen/round-trip: load the saved state if any
  const d = await (await fetch('/api/doc/post-mortem')).json();
  if (d.html) editor.innerHTML = d.html;
  refreshAtts(d.attachments);
})();
</script></body></html>`;

http.createServer(async (req, res) => {
  const u = new URL(req.url, 'http://localhost');
  if (req.method === 'GET' && u.pathname === '/') {
    res.writeHead(200, { 'Content-Type': 'text/html; charset=utf-8' });
    return res.end(PAGE);
  }
  if (req.method === 'POST' && u.pathname === '/api/attach') {
    try {
      const b = await readBody(req);
      const name = String(b.name || 'attachment.bin').replace(/[^a-zA-Z0-9._-]/g, '_');
      const payload = Buffer.from(b.bytes || []);
      const sha = crypto.createHash('sha256').update(payload).digest('hex');
      fs.writeFileSync(path.join(STORE_DIR, name), payload);
      const doc = docs['post-mortem'];
      doc.attachments = doc.attachments.filter((a) => a.name !== name);
      doc.attachments.push({ name, size: payload.length, sha });
      return jws(res, 200, { ok: true, name, size: payload.length, sha, attachments: doc.attachments });
    } catch (e) {
      return jws(res, 200, { error: 'attach failed: ' + e.message });
    }
  }
  if (req.method === 'GET' && u.pathname.startsWith('/api/attachment/')) {
    const name = u.pathname.slice('/api/attachment/'.length).replace(/[^a-zA-Z0-9._-]/g, '');
    const f = path.join(STORE_DIR, name);
    if (!fs.existsSync(f)) { res.writeHead(404); return res.end('nf'); }
    res.writeHead(200, { 'Content-Type': 'application/octet-stream' });
    return res.end(fs.readFileSync(f));
  }
  if (req.method === 'POST' && u.pathname === '/api/save') {
    const b = await readBody(req);
    docs[b.docId || 'post-mortem'] = { html: String(b.html || ''), attachments: (docs[b.docId || 'post-mortem'] || {}).attachments || [] };
    return jws(res, 200, { ok: true });
  }
  if (req.method === 'GET' && u.pathname.startsWith('/api/doc/')) {
    const id = u.pathname.slice('/api/doc/'.length) || 'post-mortem';
    return jws(res, 200, docs[id] || { html: '', attachments: [] });
  }
  jws(res, 404, { error: 'not found' });
}).listen(PORT, '127.0.0.1', () => console.log('TeamPad on http://127.0.0.1:' + PORT + '/'));
