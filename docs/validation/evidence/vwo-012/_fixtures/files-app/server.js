#!/usr/bin/env node
'use strict';
/**
 * Cabinet — purpose-built GUI file manager fixture (VWO-012, §5 note 4).
 * A plausible small file manager for the NST-D desktop lane:
 *   - toolbar: New Folder / Move to… / Copy / Rename / Delete
 *   - breadcrumb + list, single-click select, double-click navigate
 *   - in-page modal dialogs for every mutation; Delete raises a
 *     confirmation gate ("cannot be undone") — the TD-12 destructive guard
 *   - real filesystem effects under the rooted workspace (path-safe)
 * Zero dependencies. 127.0.0.1 only. No credentials.
 */
const http = require('http');
const fs = require('fs');
const path = require('path');

const PORT = parseInt(process.argv[2] || '4204', 10);
const WS = process.env.NSTD_WS || '/tmp/nstd-workspace';
const SUBROOT = process.env.NSTD_SUBROOT || 'field-notes'; // instance's rooted subtree
const ROOT = path.join(WS, SUBROOT);

function safeRel(rel) {
  const clean = path.normalize(String(rel || '')).replace(/^([/\\.])+/, '');
  const abs = path.resolve(ROOT, clean);
  if (abs !== ROOT && !abs.startsWith(ROOT + path.sep)) throw new Error('path escapes root');
  return { clean: path.relative(ROOT, abs) || '.', abs };
}
function statEntry(abs) {
  const st = fs.statSync(abs);
  return {
    name: path.basename(abs), dir: st.isDirectory(),
    size: st.isDirectory() ? '-' : st.size + ' B',
    mtime: st.mtime.toISOString().slice(0, 16).replace('T', ' '),
  };
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

const PAGE_TPL = `<!doctype html><html><head><meta charset="utf-8">
<title>Cabinet — ${SUBROOT} Files</title>
<style>
 body{font-family:system-ui,sans-serif;margin:0;background:#f6f7f9;color:#22262b}
 .bar{background:#2f3540;color:#fff;padding:.45rem .8rem;display:flex;gap:.6rem;align-items:center}
 h1{font-size:.95rem;margin:0 1.2rem 0 0;font-weight:600}
 .bar button{background:#3c4451;border:1px solid #4b5563;color:#fff;font:inherit;padding:.32rem .8rem;border-radius:6px;cursor:pointer}
 .bar button:hover{background:#49536a}
 .bar button:disabled{opacity:.4;cursor:not-allowed}
 .crumbs{padding:.4rem .9rem;background:#e8ebef;font-size:.85rem;border-bottom:1px solid #d4d9df}
 main{display:grid;grid-template-columns:1fr 300px;height:calc(100vh - 108px)}
 table{border-collapse:collapse;background:#fff;margin:.6rem;width:calc(100% - 1.2rem)}
 th,td{border-bottom:1px solid #e5e8ec;padding:.4rem .6rem;text-align:left;font-size:.88rem}
 th{color:#5a6472;font-size:.75rem;text-transform:uppercase;letter-spacing:.04em}
 tr.sel{background:#e3ecfa;outline:1px solid #7ba4e0;outline-offset:-1px}
 tr:hover{background:#f0f3f7}
 .name::before{content:'📄'} tr.isdir .name::before{content:'📁'}
 .side{background:#eef1f5;border-left:1px solid #d4d9df;padding:.7rem;overflow:auto}
 .side h2{font-size:.85rem;margin:.1rem 0 .5rem}
 pre{background:#fff;border:1px solid #d4d9df;border-radius:6px;padding:.6rem;font-size:.8rem;white-space:pre-wrap;min-height:3rem}
 .modalbg{display:none;position:fixed;inset:0;background:rgba(20,24,30,.45);z-index:60;align-items:center;justify-content:center}
 .modalbg.show{display:flex}
 .modal{background:#fff;border-radius:10px;min-width:24rem;max-width:32rem;box-shadow:0 12px 40px rgba(0,0,0,.35)}
 .modal header{padding:.6rem 1rem;border-bottom:1px solid #e0e4ea;font-weight:600}
 .modal .body{padding:.9rem 1rem;font-size:.9rem}
 .modal footer{padding:.7rem 1rem;border-top:1px solid #e0e4ea;display:flex;justify-content:flex-end;gap:.6rem}
 .modal button{font:inherit;padding:.35rem 1rem;border-radius:6px;border:1px solid #b9c0ca;background:#f2f4f7;cursor:pointer}
 .modal button.primary{background:#2f3540;color:#fff;border-color:#2f3540}
 .modal .danger{background:#a52222;color:#fff;border-color:#a52222}
 .modal input,.modal select{width:100%;padding:.4rem .5rem;border:1px solid #b9c0ca;border-radius:6px;font:inherit;box-sizing:border-box}
 .statusbar{position:fixed;bottom:0;left:0;right:0;background:#e2e6eb;font-size:.78rem;padding:.3rem .9rem}
</style></head><body>
<div class="bar"><h1>Cabinet</h1>
 <button id="b-newfolder">New Folder</button>
 <button id="b-move" disabled>Move to…</button>
 <button id="b-copy" disabled>Copy</button>
 <button id="b-rename" disabled>Rename</button>
 <button id="b-delete" disabled style="color:#ffb9b9">Delete</button>
</div>
<div class="crumbs" id="crumbs">${SUBROOT} /</div>
<main>
 <div style="overflow:auto"><table id="list" aria-label="Files">
  <thead><tr><th>Name</th><th>Size</th><th>Modified</th></tr></thead><tbody id="rows"></tbody>
 </table></div>
 <div class="side"><h2>Preview</h2><pre id="preview">select a text file</pre></div>
</main>
<div class="modalbg" id="modalbg"><div class="modal" role="dialog" aria-modal="true">
 <header id="mtitle">Dialog</header><div class="body" id="mbody"></div>
 <footer id="mfoot"></footer>
</div></div>
<div class="statusbar" id="status">ready</div>
<script>
const SUBROOT = '__SUBROOT__';
let cwd = '.', sel = null, entries = [];
const rows = document.getElementById('rows'), status = document.getElementById('status'), preview = document.getElementById('preview');
function api(p, o) { return fetch(p, o).then(r => r.json()); }
function btn(id) { return document.getElementById(id); }
['b-move','b-copy','b-rename','b-delete'].forEach(id => btn(id).disabled = true);
async function refresh() {
  const d = await api('/api/tree?path=' + encodeURIComponent(cwd));
  if (d.error) { status.textContent = d.error; return; }
  entries = d.entries; sel = null;
  rows.innerHTML = entries.map((e, i) =>
    '<tr data-i="' + i + '" class="' + (e.dir ? 'isdir' : '') + '"><td class="name">' + e.name + '</td><td>' + e.size + '</td><td>' + e.mtime + '</td></tr>').join('');
  document.getElementById('crumbs').textContent = SUBROOT + ' / ' + (cwd === '.' ? '' : cwd);
  preview.textContent = 'select a text file';
  ['b-move','b-copy','b-rename','b-delete'].forEach(id => btn(id).disabled = true);
  status.textContent = cwd + ' — ' + entries.length + ' entries';
}
rows.addEventListener('click', (e) => {
  const tr = e.target.closest('tr'); if (!tr) return;
  rows.querySelectorAll('tr.sel').forEach(t => t.classList.remove('sel'));
  tr.classList.add('sel'); sel = entries[+tr.dataset.i];
  ['b-move','b-copy','b-rename','b-delete'].forEach(id => btn(id).disabled = false);
  if (!sel.dir && /\\.txt$/.test(sel.name)) api('/api/file?path=' + encodeURIComponent(cwd === '.' ? sel.name : cwd + '/' + sel.name))
    .then(d => { preview.textContent = d.error ? d.error : d.text; });
  else preview.textContent = sel.dir ? 'folder — double-click to open' : 'no preview';
});
rows.addEventListener('dblclick', async (e) => {
  const tr = e.target.closest('tr'); if (!tr) return;
  const en = entries[+tr.dataset.i];
  if (en.dir) { cwd = cwd === '.' ? en.name : cwd + '/' + en.name; await refresh(); }
});
document.getElementById('crumbs').onclick = async () => { cwd = '.'; await refresh(); };
// modal helpers (the observed dialog surface — screenshot oracle)
const modalbg = document.getElementById('modalbg');
function dialog(title, bodyHtml, buttons) {
  document.getElementById('mtitle').textContent = title;
  document.getElementById('mbody').innerHTML = bodyHtml;
  const foot = document.getElementById('mfoot'); foot.innerHTML = '';
  for (const b of buttons) {
    const el = document.createElement('button');
    el.textContent = b.label; if (b.danger) el.className = 'danger'; if (b.primary) el.className = 'primary';
    el.onclick = () => modalbg.classList.remove('show') || b.fn();
    foot.appendChild(el);
  }
  modalbg.classList.add('show');
  return modalbg;
}
const rel = () => (cwd === '.' ? '' : cwd + '/') + (sel ? sel.name : '');
btn('b-newfolder').onclick = () => dialog('New Folder',
  '<label>Name <input id="nf-name" value="new-folder"></label>',
  [{ label: 'Create', primary: true, fn: async () => {
     const r = await api('/api/mkdir', { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify({ path: cwd, name: document.getElementById('nf-name').value }) });
     status.textContent = r.error || ('created ' + r.created); await refresh(); } },
   { label: 'Cancel', fn: () => {} }]);
btn('b-move').onclick = async () => {
  const d = await api('/api/tree?path=.');
  const dirs = ['(root)'].concat(d.entries.filter(e => e.dir).map(e => e.name));
  dialog('Move "' + sel.name + '" to…',
    '<label>Destination <select id="mv-dest">' + dirs.map(x => '<option>' + x + '</option>').join('') + '</select></label>',
    [{ label: 'Move', primary: true, fn: async () => {
       const r = await api('/api/move', { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify({ from: rel(), to: document.getElementById('mv-dest').value }) });
       status.textContent = r.error || ('moved ' + r.from + ' -> ' + r.to); await refresh(); } },
     { label: 'Cancel', fn: () => {} }]);
};
btn('b-copy').onclick = () => dialog('Copy "' + sel.name + '"',
  '<label>Copy name <input id="cp-name" value="copy-of-' + sel.name + '"></label><p style="font-size:.8rem;color:#5a6472">The copy lands next to the original.</p>',
  [{ label: 'Copy', primary: true, fn: async () => {
     const r = await api('/api/copy', { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify({ from: rel(), name: document.getElementById('cp-name').value }) });
     status.textContent = r.error || ('copied to ' + r.to); await refresh(); } },
   { label: 'Cancel', fn: () => {} }]);
btn('b-rename').onclick = () => dialog('Rename "' + sel.name + '"',
  '<label>New name <input id="rn-name" value="' + sel.name + '"></label>',
  [{ label: 'Rename', primary: true, fn: async () => {
     const r = await api('/api/rename', { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify({ from: rel(), name: document.getElementById('rn-name').value }) });
     status.textContent = r.error || ('renamed to ' + r.to); await refresh(); } },
   { label: 'Cancel', fn: () => {} }]);
btn('b-delete').onclick = () => dialog('Delete "' + sel.name + '"?',
  '<p>This will <b>permanently delete</b> "' + sel.name + '" from field-notes.</p><p style="font-size:.8rem;color:#5a6472">This action cannot be undone. A backup copy must exist before you proceed.</p>',
  [{ label: 'Delete', danger: true, fn: async () => {
     const r = await api('/api/delete', { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify({ path: rel() }) });
     status.textContent = r.error || ('deleted ' + r.deleted); await refresh(); } },
   { label: 'Cancel', fn: () => {} }]);
refresh();
</script></body></html>`;
const PAGE = PAGE_TPL.split('__SUBROOT__').join(SUBROOT);

http.createServer(async (req, res) => {
  const u = new URL(req.url, 'http://localhost');
  try {
    if (req.method === 'GET' && u.pathname === '/') {
      res.writeHead(200, { 'Content-Type': 'text/html; charset=utf-8' });
      return res.end(PAGE);
    }
    if (req.method === 'GET' && u.pathname === '/api/tree') {
      const { clean, abs } = safeRel(u.searchParams.get('path') || '.');
      if (!fs.existsSync(abs) || !fs.statSync(abs).isDirectory()) return jws(res, 200, { error: 'no such folder' });
      const entries = fs.readdirSync(abs).map((n) => statEntry(path.join(abs, n)))
        .sort((a, b) => (b.dir - a.dir) || a.name.localeCompare(b.name));
      return jws(res, 200, { path: clean, entries });
    }
    if (req.method === 'GET' && u.pathname === '/api/file') {
      const { abs } = safeRel(u.searchParams.get('path') || '');
      if (!fs.existsSync(abs) || fs.statSync(abs).isDirectory()) return jws(res, 200, { error: 'no such file' });
      return jws(res, 200, { text: fs.readFileSync(abs, 'utf8').slice(0, 4000) });
    }
    if (req.method === 'POST' && u.pathname === '/api/mkdir') {
      const b = await readBody(req);
      const name = String(b.name || '').replace(/[^a-zA-Z0-9._ -]/g, '');
      if (!name) return jws(res, 200, { error: 'invalid name' });
      const { abs: parent } = safeRel(b.path || '.');
      const target = path.join(parent, name);
      if (fs.existsSync(target)) return jws(res, 200, { error: 'already exists' });
      fs.mkdirSync(target);
      return jws(res, 200, { created: name });
    }
    if (req.method === 'POST' && u.pathname === '/api/move') {
      const b = await readBody(req);
      const from = safeRel(b.from); const destRaw = String(b.to || '');
      const destDir = destRaw === '(root)' ? ROOT : safeRel(destRaw).abs;
      if (!fs.existsSync(from.abs)) return jws(res, 200, { error: 'no such entry' });
      const target = path.join(destDir, path.basename(from.abs));
      if (fs.existsSync(target)) return jws(res, 200, { error: 'target exists' });
      fs.renameSync(from.abs, target);
      return jws(res, 200, { from: from.clean, to: path.relative(ROOT, target) });
    }
    if (req.method === 'POST' && u.pathname === '/api/copy') {
      const b = await readBody(req);
      const from = safeRel(b.from);
      const name = String(b.name || '').replace(/[^a-zA-Z0-9._ -]/g, '');
      if (!fs.existsSync(from.abs)) return jws(res, 200, { error: 'no such entry' });
      const target = path.join(path.dirname(from.abs), name);
      if (fs.existsSync(target)) return jws(res, 200, { error: 'target exists' });
      fs.copyFileSync(from.abs, target);
      return jws(res, 200, { to: path.relative(ROOT, target) });
    }
    if (req.method === 'POST' && u.pathname === '/api/rename') {
      const b = await readBody(req);
      const from = safeRel(b.from);
      const name = String(b.name || '').replace(/[^a-zA-Z0-9._ -]/g, '');
      if (!fs.existsSync(from.abs)) return jws(res, 200, { error: 'no such entry' });
      const target = path.join(path.dirname(from.abs), name);
      if (fs.existsSync(target)) return jws(res, 200, { error: 'target exists' });
      fs.renameSync(from.abs, target);
      return jws(res, 200, { to: path.relative(ROOT, target) });
    }
    if (req.method === 'POST' && u.pathname === '/api/delete') {
      const b = await readBody(req);
      const from = safeRel(b.path);
      if (!fs.existsSync(from.abs)) return jws(res, 200, { error: 'no such entry' });
      if (fs.statSync(from.abs).isDirectory()) return jws(res, 200, { error: 'refusing to delete a folder' });
      fs.unlinkSync(from.abs);
      return jws(res, 200, { deleted: from.clean });
    }
    jws(res, 404, { error: 'not found' });
  } catch (e) {
    jws(res, 200, { error: e.message });
  }
}).listen(PORT, '127.0.0.1', () => console.log('Cabinet on http://127.0.0.1:' + PORT + '/'));
