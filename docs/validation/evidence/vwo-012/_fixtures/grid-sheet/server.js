#!/usr/bin/env node
'use strict';
/**
 * LedgerGrid — purpose-built GUI spreadsheet fixture (VWO-012, §5 note 4).
 * A plausible small expense-sheet product for the NST-D desktop lane:
 *   - 6x8 grid (A1..F8) with cell selection, formula bar, receipt side panel
 *   - client cell state; server independently evaluates =SUM(B2:B6) at
 *     export time (oracle for the total) and writes a real CSV download
 * Zero dependencies. 127.0.0.1 only. No credentials.
 */
const http = require('http');

const PORT = parseInt(process.argv[2] || '4202', 10);

const RECEIPTS = [
  { id: 'r-101', vendor: 'Coral Hardware', item: 'guardrail straps', amount: 48.5, date: '2026-09-08' },
  { id: 'r-102', vendor: 'Metro Concrete', item: 'slab pour (L12)', amount: 612.0, date: '2026-09-09' },
  { id: 'r-103', vendor: 'Rebar Supply Co', item: 'rebar L13 delivery', amount: 275.25, date: '2026-09-10' },
  { id: 'r-104', vendor: 'LiftIt Crane Svcs', item: 'crane B recert', amount: 340.0, date: '2026-09-11' },
  { id: 'r-105', vendor: 'Coral Hardware', item: 'formwork brackets', amount: 96.75, date: '2026-09-12' },
];

function jws(res, code, obj) {
  const body = JSON.stringify(obj);
  res.writeHead(code, { 'Content-Type': 'application/json', 'Content-Length': Buffer.byteLength(body) });
  res.end(body);
}

const PAGE = `<!doctype html><html><head><meta charset="utf-8">
<title>LedgerGrid — Expense Sheet</title>
<style>
 body{font-family:system-ui,sans-serif;margin:0;background:#f2f5f3;color:#1c2b25}
 .bar{background:#1e4034;color:#fff;padding:.4rem .8rem;display:flex;gap:1rem;align-items:center}
 h1{font-size:1rem;margin:0;font-weight:600}
 .bar button{background:#2c5a48;border:none;color:#fff;font:inherit;padding:.3rem .8rem;border-radius:5px;cursor:pointer}
 .bar button:hover{background:#37725c}
 .fbar{display:flex;gap:.5rem;padding:.4rem .8rem;background:#e2ebe6;border-bottom:1px solid #c4d4cb;align-items:center}
 .fbar .cellref{font-family:ui-monospace,monospace;font-weight:700;min-width:3rem}
 .fbar input{flex:1;padding:.3rem .5rem;border:1px solid #b7c9bf;border-radius:4px;font:inherit}
 main{display:grid;grid-template-columns:1fr 260px;gap:0;height:calc(100vh - 92px)}
 table{border-collapse:collapse;background:#fff;margin:.6rem}
 th,td{border:1px solid #cfdcd3;padding:.25rem .5rem;min-width:7rem;text-align:left;font-size:.9rem}
 th{background:#e6efe9;font-family:ui-monospace,monospace;font-size:.75rem}
 td.sel{outline:2px solid #1e4034;outline-offset:-2px}
 td.editing{background:#fffbe8}
 .side{background:#eef3ef;border-left:1px solid #c4d4cb;padding:.7rem;overflow:auto}
 .side h2{font-size:.9rem;margin:.2rem 0 .5rem}
 .rec{background:#fff;border:1px solid #d4e0d8;border-radius:6px;padding:.4rem .6rem;margin-bottom:.45rem;font-size:.82rem}
 .rec b{display:block}
 .hint{font-size:.75rem;color:#48604f;margin-top:.6rem}
 .status{padding:.25rem .8rem;background:#dbe7df;font-size:.78rem}
</style></head><body>
<div class="bar"><h1>LedgerGrid</h1><button id="export">Export CSV…</button><span style="margin-left:auto;font-size:.8rem">expense-sheet.lg · week 37</span></div>
<div class="fbar"><span class="cellref" id="cellref">A1</span><input id="finput" aria-label="Formula bar" placeholder="Type a value or =SUM(B2:B6)" /></div>
<main>
 <div style="overflow:auto"><table id="grid" aria-label="Expense grid"></table></div>
 <div class="side"><h2>Receipts this week</h2><div id="recs"></div>
  <p class="hint">Summary cell B7 expects <code>=SUM(B2:B6)</code>. Export writes one CSV: the five entry rows plus the computed total.</p></div>
</main>
<div class="status" id="status">ready</div>
<script>
const COLS = ['A','B','C','D','E','F'], ROWS = 8;
let cells = {}, sel = 'A1';
const grid = document.getElementById('grid');
let head = '<tr><th></th>' + COLS.map(c => '<th>' + c + '</th>').join('') + '</tr>';
for (let r = 1; r <= ROWS; r++) {
  head += '<tr><th>' + r + '</th>' + COLS.map(c => '<td id="' + c + r + '" data-cell="' + c + r + '"></td>').join('') + '</tr>';
}
grid.innerHTML = head;
const finput = document.getElementById('finput'), cellref = document.getElementById('cellref'), status = document.getElementById('status');
function refresh() { for (const id in cells) { const td = document.getElementById(id); if (td) td.textContent = cells[id]; } }
function setSel(id) { document.querySelectorAll('td.sel').forEach(t => t.classList.remove('sel')); sel = id; const td = document.getElementById(id); if (td) td.classList.add('sel'); cellref.textContent = id; finput.value = cells[id] || ''; }
grid.addEventListener('click', (e) => { const td = e.target.closest('td'); if (td && td.dataset.cell) setSel(td.dataset.cell); });
finput.addEventListener('keydown', (e) => {
  if (e.key === 'Enter') {
    cells[sel] = finput.value; refresh(); status.textContent = sel + ' = ' + (finput.value || '(empty)');
    e.preventDefault();
  }
});
(async () => { const rs = await (await fetch('/api/receipts')).json();
  document.getElementById('recs').innerHTML = rs.map(r =>
    '<div class="rec"><b>' + r.vendor + '</b>' + r.item + ' — $' + r.amount.toFixed(2) + '<br><span style="color:#48604f">' + r.date + ' · ' + r.id + '</span></div>').join('');
})();
document.getElementById('export').onclick = () => {
  fetch('/api/export', { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify({ cells }) })
    .then(r => r.json()).then(d => {
      if (d.error) { alert(d.error); status.textContent = 'export blocked: ' + d.error; return; }
      const a = document.createElement('a'); a.href = d.url; a.download = d.name; a.click();
      status.textContent = 'exported ' + d.name + ' (total ' + d.total + ')';
    });
};
setSel('A1'); refresh();
</script></body></html>`;

const exports_pending = {};

http.createServer(async (req, res) => {
  const u = new URL(req.url, 'http://localhost');
  if (req.method === 'GET' && u.pathname === '/') {
    res.writeHead(200, { 'Content-Type': 'text/html; charset=utf-8' });
    return res.end(PAGE);
  }
  if (req.method === 'GET' && u.pathname === '/api/receipts') return jws(res, 200, RECEIPTS);
  if (req.method === 'POST' && u.pathname === '/api/export') {
    let b = ''; req.on('data', (c) => (b += c)); req.on('end', () => {
      let cells = {}; try { cells = JSON.parse(b || '{}').cells || {}; } catch {}
      const rows = [];
      for (let r = 2; r <= 6; r++) {
        rows.push([cells['A' + r] || '', cells['B' + r] || '', cells['C' + r] || '']);
      }
      const sumCell = String(cells['B7'] || '');
      const m = /^=SUM\(B2:B6\)$/i.exec(sumCell.replace(/\s+/g, ''));
      if (!m) return jws(res, 200, { error: 'Summary cell B7 must contain =SUM(B2:B6) — found: "' + sumCell + '"' });
      const amounts = rows.map((r) => parseFloat(r[1]));
      if (amounts.some((a) => !isFinite(a))) return jws(res, 200, { error: 'Rows B2..B6 must all be numeric amounts' });
      const total = amounts.reduce((a, x) => a + x, 0);
      const esc = (v) => '"' + String(v).replace(/"/g, '""') + '"';
      let csv = 'vendor,amount,note\n';
      for (const r of rows) csv += esc(r[0]) + ',' + esc(r[1]) + ',' + esc(r[2]) + '\n';
      csv += esc('TOTAL') + ',' + esc(total.toFixed(2)) + ',' + esc('=SUM(B2:B6)') + '\n';
      const token = 'exp-' + Date.now();
      exports_pending[token] = csv;
      return jws(res, 200, { ok: true, url: '/api/export-file/' + token, name: 'expense-sheet-week37.csv', total: total.toFixed(2) });
    });
    return;
  }
  if (req.method === 'GET' && u.pathname.startsWith('/api/export-file/')) {
    const token = u.pathname.slice('/api/export-file/'.length).replace(/[^a-z0-9-]/gi, '');
    const csv = exports_pending[token];
    if (!csv) { res.writeHead(404); return res.end('gone'); }
    res.writeHead(200, { 'Content-Type': 'text/csv', 'Content-Disposition': 'attachment; filename="expense-sheet-week37.csv"' });
    return res.end(csv);
  }
  jws(res, 404, { error: 'not found' });
}).listen(PORT, '127.0.0.1', () => console.log('LedgerGrid on http://127.0.0.1:' + PORT + '/'));
