#!/usr/bin/env node
'use strict';
/**
 * NotaryPad — purpose-built GUI notes editor fixture (VWO-012, §5 note 4).
 * A plausible small notes product for the NST-D desktop lane:
 *   - File menu: New / Open… / Save / Save As… / Exit
 *   - dirty tracking + server-side autosave (durable draft state)
 *   - crash-recovery banner on reload when an autosave exists
 *   - Save As… serves the buffer as a download (native browser save dialog)
 *   - Save writes into the team-notes workspace directory (D007 team path)
 *   - Exit with unsaved changes raises the close-confirmation (beforeunload)
 * Zero dependencies. Serves on 127.0.0.1 only. No credentials — demo tool.
 */
const http = require('http');
const fs = require('fs');
const path = require('path');

const PORT = parseInt(process.argv[2] || '4201', 10);
const WS = process.env.NSTD_WS || '/tmp/nstd-workspace';
const TEAM_DIR = path.join(WS, 'team-notes');
const AUTOSAVE_DIR = path.join(WS, '.autosave');
for (const d of [TEAM_DIR, AUTOSAVE_DIR]) fs.mkdirSync(d, { recursive: true });

const state = { docs: {} }; // docId -> { text, savedAt, savedName }

function jws(res, code, obj) {
  const body = JSON.stringify(obj);
  res.writeHead(code, { 'Content-Type': 'application/json', 'Content-Length': Buffer.byteLength(body) });
  res.end(body);
}
function readBody(req) {
  return new Promise((resolve) => {
    let b = '';
    req.on('data', (c) => { b += c; });
    req.on('end', () => { try { resolve(JSON.parse(b || '{}')); } catch { resolve({}); } });
  });
}
function safeName(n) {
  return String(n || '').replace(/[^a-zA-Z0-9._ -]/g, '').slice(0, 80);
}

const PAGE = `<!doctype html><html><head><meta charset="utf-8">
<title>NotaryPad — Notes Editor</title>
<style>
 body{font-family:system-ui,sans-serif;margin:0;background:#f4f1ea;color:#232323}
 .bar{background:#3d3830;color:#fff;padding:.4rem .8rem;display:flex;gap:1rem;align-items:center}
 .bar h1{font-size:1rem;margin:0;font-weight:600;letter-spacing:.03em}
 .menu{position:relative}
 .menu>button{background:none;border:none;color:#fff;font:inherit;cursor:pointer;padding:.25rem .6rem;border-radius:4px}
 .menu>button:hover,.menu.open>button{background:#5a5348}
 .dropdown{display:none;position:absolute;top:100%;left:0;background:#fff;color:#232323;border:1px solid #c9c2b4;border-radius:6px;min-width:12rem;box-shadow:0 6px 18px rgba(0,0,0,.25);z-index:40}
 .menu.open .dropdown{display:block}
 .dropdown button{display:block;width:100%;text-align:left;background:none;border:none;font:inherit;padding:.5rem .9rem;cursor:pointer}
 .dropdown button:hover{background:#efe9dc}
 .docname{margin-left:auto;font-size:.85rem;opacity:.85}
 main{display:grid;grid-template-rows:auto 1fr auto;height:calc(100vh - 44px)}
 .status{padding:.3rem .9rem;background:#e9e4d8;font-size:.8rem;display:flex;gap:1.2rem;border-bottom:1px solid #d5cfbf}
 #buffer{width:100%;height:100%;border:none;resize:none;padding:1rem 1.2rem;font:1rem/1.6 ui-monospace,monospace;background:#fffdf6;color:#232323;outline:none;box-sizing:border-box}
 .recover{display:none;background:#fdf0d5;border-bottom:1px solid #f0d9a8;padding:.5rem .9rem;font-size:.9rem}
 .appmodalbg{display:none;position:fixed;inset:0;background:rgba(30,26,20,.45);z-index:60;align-items:center;justify-content:center}
 .appmodalbg.show{display:flex}
 .appmodal{background:#fffdf6;border:1px solid #c9c2b4;border-radius:10px;min-width:26rem;max-width:34rem;box-shadow:0 12px 40px rgba(0,0,0,.35)}
 .appmodal header{padding:.6rem 1rem;border-bottom:1px solid #e0dacb;font-weight:600}
 .appmodal .body{padding:.9rem 1rem;font-size:.92rem}
 .appmodal footer{padding:.7rem 1rem;border-top:1px solid #e0dacb;display:flex;justify-content:flex-end;gap:.6rem}
 .appmodal button{font:inherit;padding:.35rem 1rem;border-radius:6px;border:1px solid #b9b09e;background:#f2eee4;cursor:pointer}
 .appmodal button.primary{background:#3d3830;color:#fff;border-color:#3d3830}
 .appmodal button.danger{background:#a52222;color:#fff;border-color:#a52222}
 .appmodal input{width:100%;padding:.4rem .5rem;border:1px solid #b9b09e;border-radius:6px;font:inherit;box-sizing:border-box}
 .recover button{margin-left:.6rem}
 .unsaved{color:#b45309;font-weight:600}.saved{color:#14532d}
</style></head><body>
<div class="bar">
 <h1>NotaryPad</h1>
 <div class="menu" id="filemenu"><button id="filebtn" aria-haspopup="true">File</button>
  <div class="dropdown" role="menu">
   <button id="m-new" role="menuitem">New</button>
   <button id="m-open" role="menuitem">Open…</button>
   <button id="m-save" role="menuitem">Save</button>
   <button id="m-saveas" role="menuitem">Save As…</button>
   <button id="m-exit" role="menuitem">Exit</button>
  </div></div>
 <div class="docname" id="docname">untitled.txt</div>
</div>
<div class="status"><span id="savestate" class="saved">saved</span><span id="autosaveAt">autosave: —</span><span>NotaryPad 1.4 · workspace: team-notes</span></div>
<div class="recover" id="recover">Draft recovered from autosave <span id="recTs"></span><button id="restore">Restore draft</button><button id="discard">Discard</button></div>
<main><textarea id="buffer" spellcheck="false" aria-label="Note buffer"></textarea></main>
<div class="appmodalbg" id="saveasModal" aria-hidden="true"><div class="appmodal" role="dialog" aria-modal="true" aria-label="Save note as">
 <header>Save note as…</header>
 <div class="body"><label>Note name <input id="saveasName" value="untitled.txt"></label></div>
 <footer><button id="saveasCancel">Cancel</button><button id="saveasOk" class="primary">Save</button></footer>
</div></div>
<div class="appmodalbg" id="exitModal" aria-hidden="true"><div class="appmodal" role="dialog" aria-modal="true" aria-label="Unsaved changes">
 <header>Unsaved changes</header>
 <div class="body">The note has unsaved changes. Close without saving?</div>
 <footer><button id="exitCancel">Keep editing</button><button id="exitDiscard" class="danger">Close without saving</button></footer>
</div></div>
<script>
const DOC = new URLSearchParams(location.search).get('doc') || 'note-' + Math.random().toString(36).slice(2, 8);
history.replaceState(null, '', '/?doc=' + DOC);
let buffer = document.getElementById('buffer'), savestate = document.getElementById('savestate');
let docname = document.getElementById('docname'), autosaveAt = document.getElementById('autosaveAt');
let dirty = false, savedName = 'untitled.txt', lastSaveText = '';
function markDirty(){ if(!dirty){ dirty = true; savestate.textContent = 'unsaved changes'; savestate.className = 'unsaved'; } }
function markSaved(name){ dirty = false; if(name){ savedName = name; docname.textContent = name; } savestate.textContent = 'saved'; savestate.className = 'saved'; }
buffer.addEventListener('input', markDirty);
window.addEventListener('beforeunload', (e) => { if (dirty) { e.preventDefault(); e.returnValue = ''; } });
// autosave loop (2s when dirty) — durable server-side draft state
setInterval(async () => { if (!dirty) return;
  await fetch('/api/autosave', {method:'POST', headers:{'Content-Type':'application/json'},
    body: JSON.stringify({docId: DOC, text: buffer.value})});
  autosaveAt.textContent = 'autosave: ' + new Date().toISOString().slice(11, 19);
}, 2000);
// crash-recovery banner: autosave exists and differs from empty buffer start
(async () => { const r = await fetch('/api/autosave?docId=' + DOC); const d = await r.json();
  if (d.found && d.text && d.text !== '') {
    const el = document.getElementById('recover'); el.style.display = 'block';
    document.getElementById('recTs').textContent = d.ts;
    document.getElementById('restore').onclick = () => { buffer.value = d.text; markDirty(); el.style.display = 'none'; };
    document.getElementById('discard').onclick = () => { el.style.display = 'none'; };
  } })();
// File menu
const menu = document.getElementById('filemenu');
document.getElementById('filebtn').onclick = (e) => { e.stopPropagation(); menu.classList.toggle('open'); };
document.addEventListener('click', () => menu.classList.remove('open'));
document.getElementById('m-new').onclick = () => { if (dirty && !confirm('Discard unsaved changes?')) return; buffer.value = ''; markSaved('untitled.txt'); };
document.getElementById('m-open').onclick = async () => {
  const names = await (await fetch('/api/notes')).json();
  if (!names.length) { alert('No saved notes yet.'); return; }
  const name = prompt('Open note:\\n' + names.join('\\n'));
  if (!name) return;
  const d = await (await fetch('/api/notes/' + encodeURIComponent(name))).json();
  if (d.error) { alert(d.error); return; }
  buffer.value = d.text; markSaved(name);
};
document.getElementById('m-save').onclick = async () => {
  const d = await (await fetch('/api/save', {method:'POST', headers:{'Content-Type':'application/json'},
    body: JSON.stringify({docId: DOC, name: savedName, text: buffer.value})})).json();
  if (d.error) { alert(d.error); return; } markSaved(d.name);
};
document.getElementById('m-saveas').onclick = () => {
  const m = document.getElementById('saveasModal');
  document.getElementById('saveasName').value = savedName;
  m.classList.add('show'); m.setAttribute('aria-hidden', 'false');
  document.getElementById('saveasName').focus();
};
document.getElementById('saveasCancel').onclick = () => { const m = document.getElementById('saveasModal'); m.classList.remove('show'); m.setAttribute('aria-hidden', 'true'); };
document.getElementById('saveasOk').onclick = () => {
  const name = document.getElementById('saveasName').value.trim() || 'untitled.txt';
  savedName = name;
  const m = document.getElementById('saveasModal'); m.classList.remove('show'); m.setAttribute('aria-hidden', 'true');
  // durable server-side write under the new name (team-notes/<name>)
  fetch('/api/save', {method:'POST', headers:{'Content-Type':'application/json'},
    body: JSON.stringify({docId: DOC, name: name, text: buffer.value})})
    .then(r => r.json()).then(d => { markSaved(d.name || name); });
};
document.getElementById('m-exit').onclick = () => {
  if (!dirty) { window.location.href = '/?doc=' + DOC + '&exited=1'; return; }
  const m = document.getElementById('exitModal');
  m.classList.add('show'); m.setAttribute('aria-hidden', 'false');
};
document.getElementById('exitCancel').onclick = () => { const m = document.getElementById('exitModal'); m.classList.remove('show'); m.setAttribute('aria-hidden', 'true'); };
document.getElementById('exitDiscard').onclick = () => { window.location.href = '/?doc=' + DOC + '&exited=1'; };
</script></body></html>`;

http.createServer(async (req, res) => {
  const u = new URL(req.url, 'http://localhost');
  if (req.method === 'GET' && u.pathname === '/') {
    res.writeHead(200, { 'Content-Type': 'text/html; charset=utf-8' });
    return res.end(PAGE);
  }
  if (req.method === 'GET' && u.pathname === '/api/autosave') {
    const docId = u.searchParams.get('docId') || '';
    const f = path.join(AUTOSAVE_DIR, safeName(docId) + '.json');
    if (!fs.existsSync(f)) return jws(res, 200, { found: false });
    const d = JSON.parse(fs.readFileSync(f, 'utf8'));
    return jws(res, 200, { found: true, text: d.text, ts: d.ts });
  }
  if (req.method === 'POST' && u.pathname === '/api/autosave') {
    const b = await readBody(req);
    const docId = safeName(b.docId);
    const rec = { text: String(b.text || ''), ts: new Date().toISOString() };
    fs.writeFileSync(path.join(AUTOSAVE_DIR, docId + '.json'), JSON.stringify(rec));
    return jws(res, 200, { ok: true, ts: rec.ts });
  }
  if (req.method === 'POST' && u.pathname === '/api/save') {
    const b = await readBody(req);
    let name = safeName(b.name) || 'untitled.txt';
    if (!/\.txt$/.test(name)) name = name + '.txt';
    fs.writeFileSync(path.join(TEAM_DIR, name), String(b.text || ''));
    state.docs[safeName(b.docId)] = { text: String(b.text || ''), savedAt: new Date().toISOString(), savedName: name };
    return jws(res, 200, { ok: true, name });
  }
  if (req.method === 'GET' && u.pathname === '/api/notes') {
    const names = fs.readdirSync(TEAM_DIR).filter((f) => f.endsWith('.txt') && f !== 'README.txt').sort();
    return jws(res, 200, names);
  }
  if (req.method === 'GET' && u.pathname.startsWith('/api/notes/')) {
    const name = safeName(decodeURIComponent(u.pathname.slice('/api/notes/'.length)));
    const f = path.join(TEAM_DIR, name);
    if (!fs.existsSync(f)) return jws(res, 200, { error: 'no such note: ' + name });
    return jws(res, 200, { name, text: fs.readFileSync(f, 'utf8') });
  }
  if (req.method === 'GET' && u.pathname.startsWith('/download/')) {
    const docId = safeName(u.pathname.slice('/download/'.length));
    const name = safeName(u.searchParams.get('name') || 'note.txt');
    const rec = state.docs[docId];
    const text = rec ? rec.text : '';
    res.writeHead(200, {
      'Content-Type': 'text/plain; charset=utf-8',
      'Content-Disposition': 'attachment; filename="' + name + '"',
    });
    return res.end(text);
  }
  jws(res, 404, { error: 'not found' });
}).listen(PORT, '127.0.0.1', () => console.log('NotaryPad on http://127.0.0.1:' + PORT + '/'));
