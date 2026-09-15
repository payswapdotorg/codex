#!/usr/bin/env node
'use strict';
/**
 * VendorGate — purpose-built dynamic-form fixture (VWO-013 NST-B002, §5 note 4).
 * A plausible vendor-registration product with the dynamics the task binds:
 *   - vendor type 'contractor' reveals the license field (conditional)
 *   - tax id validated inline (format VEN-<7 digits>); invalid = inline error
 *   - submit disabled until all visible fields valid
 *   - a review section that loads late (skeleton -> content) and must be awaited
 *   - submissions API records accepted + rejected attempts + a timing log
 * Zero dependencies. 127.0.0.1 only. No credentials.
 */
const http = require('http');

const PORT = parseInt(process.argv[2] || '4207', 10);
const submissions = [];   // {accepted, values, at}
const timingLog = [];     // {step, at}

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
<title>VendorGate — Vendor Registration</title>
<style>
 body{font-family:system-ui,sans-serif;margin:0;background:#f7f8fa;color:#1f2937}
 .bar{background:#334155;color:#fff;padding:.5rem .9rem}
 h1{font-size:1rem;margin:0;font-weight:600}
 main{max-width:40rem;margin:1.2rem auto;padding:0 1rem}
 .field{margin-bottom:.9rem}
 label{display:block;font-size:.82rem;font-weight:600;margin-bottom:.25rem}
 input,select{width:100%;padding:.45rem .6rem;border:1px solid #cbd5e1;border-radius:7px;font:inherit;box-sizing:border-width}
 input.err{border-color:#dc2626;background:#fef2f2}
 .hint{font-size:.75rem;color:#64748b;margin-top:.2rem}
 .inline-err{display:none;color:#dc2626;font-size:.78rem;margin-top:.2rem}
 .inline-err.show{display:block}
 .hidden{display:none}
 button{font:inherit;padding:.55rem 1.2rem;border-radius:8px;border:none;background:#94a3b8;color:#fff;cursor:not-allowed}
 button.enabled{background:#166534;cursor:pointer}
 .ok{display:none;background:#dcfce7;border:1px solid #86efac;color:#14532d;padding:.7rem 1rem;border-radius:8px;margin-top:1rem;font-weight:600}
 .ok.show{display:block}
 .skeleton{height:5rem;border-radius:8px;background:linear-gradient(90deg,#e2e8f0 25%,#f1f5f9 50%,#e2e8f0 75%);background-size:200% 100%;animation:sh 1.1s infinite;margin-top:1.2rem}
 @keyframes sh{0%{background-position:200% 0}100%{background-position:-200% 0}}
 .review{display:none;margin-top:1.2rem;border:1px solid #cbd5e1;border-radius:8px;padding:.8rem;background:#fff}
 .review.show{display:block}
 .review h2{font-size:.9rem;margin:.1rem 0 .5rem}
 .review table{width:100%;border-collapse:collapse;font-size:.85rem}
 .review td{padding:.25rem .4rem;border-bottom:1px solid #e2e8f0}
</style></head><body>
<div class="bar"><h1>VendorGate — vendor registration</h1></div>
<main>
 <div class="field"><label>Company name</label><input id="company" placeholder="Acme Trenching Ltd"></div>
 <div class="field"><label>Vendor type</label>
  <select id="vtype"><option value="">— pick —</option><option value="supplier">Supplier</option><option value="contractor">Contractor</option></select></div>
 <div class="field hidden" id="licenseField"><label>License number</label><input id="license" placeholder="LIC-000000"><div class="hint">Required for contractors (state license).</div></div>
 <div class="field"><label>Tax ID</label><input id="taxid" placeholder="VEN-0000000"><div class="inline-err" id="taxErr">Tax ID must match VEN- followed by exactly 7 digits.</div></div>
 <div class="skeleton" id="skeleton" role="status" aria-label="Loading review section"></div>
 <div class="review" id="review"><h2>Review your registration</h2><table id="reviewTable"></table></div>
 <p><button id="submit" disabled>Submit registration</button></p>
 <div class="ok" id="ok">Registration accepted — reference <span id="ref"></span>. A reviewer will contact you.</div>
</main>
<script>
const api = (p, o) => fetch(p, o).then(r => r.json());
let taxValid = false, companyValid = false, typePicked = false, licenseValid = false;
const taxid = document.getElementById('taxid'), taxErr = document.getElementById('taxErr');
const submit = document.getElementById('submit');
function validateTax() {
  const v = taxid.value.trim();
  taxValid = /^VEN-\\d{7}$/.test(v);
  taxid.classList.toggle('err', !taxValid && v.length > 0);
  taxErr.classList.toggle('show', !taxValid && v.length > 0);
  updateEnabled();
}
function validateLicense() {
  licenseValid = /^LIC-\\d{6}$/.test(document.getElementById('license').value.trim());
  updateEnabled();
}
function updateEnabled() {
  const needLicense = document.getElementById('vtype').value === 'contractor';
  const ok = companyValid && typePicked && taxValid && (!needLicense || licenseValid);
  submit.classList.toggle('enabled', ok);
  submit.disabled = !ok;
}
document.getElementById('company').addEventListener('input', function(){ companyValid = this.value.trim().length > 2; updateEnabled(); });
document.getElementById('vtype').addEventListener('change', function(){
  typePicked = this.value !== '';
  const need = this.value === 'contractor';
  document.getElementById('licenseField').classList.toggle('hidden', !need);
  if (!need) licenseValid = false;
  updateEnabled();
});
document.getElementById('license').addEventListener('input', validateLicense);
taxid.addEventListener('input', validateTax);
// delayed review section: skeleton for ~2.5s then content (must be awaited)
setTimeout(async () => {
  const t = await api('/api/timing', {method:'POST', headers:{'Content-Type':'application/json'}, body: JSON.stringify({step:'review-load-start'})});
  setTimeout(() => {
    document.getElementById('skeleton').style.display = 'none';
    document.getElementById('review').classList.add('show');
    api('/api/timing', {method:'POST', headers:{'Content-Type':'application/json'}, body: JSON.stringify({step:'review-load-done'})});
  }, 2500);
}, 400);
function reviewRow(k, v) { return '<tr><td><b>'+k+'</b></td><td>'+v+'</td></tr>'; }
function refreshReview() {
  const need = document.getElementById('vtype').value === 'contractor';
  document.getElementById('reviewTable').innerHTML =
    reviewRow('Company', document.getElementById('company').value) +
    reviewRow('Type', document.getElementById('vtype').value) +
    (need ? reviewRow('License', document.getElementById('license').value) : '') +
    reviewRow('Tax ID', taxid.value);
}
['company','vtype','license','taxid'].forEach(id => document.getElementById(id).addEventListener('change', refreshReview));
refreshReview();
submit.addEventListener('click', async () => {
  refreshReview();
  const payload = {company: document.getElementById('company').value.trim(),
    vtype: document.getElementById('vtype').value,
    license: document.getElementById('vtype').value === 'contractor' ? document.getElementById('license').value.trim() : null,
    taxid: taxid.value.trim()};
  const r = await api('/api/submit', {method:'POST', headers:{'Content-Type':'application/json'}, body: JSON.stringify(payload)});
  if (r.accepted) {
    document.getElementById('ref').textContent = r.ref;
    document.getElementById('ok').classList.add('show');
    submit.disabled = true; submit.classList.remove('enabled');
  }
});
</script></body></html>`;

http.createServer(async (req, res) => {
  const u = new URL(req.url, 'http://localhost');
  if (req.method === 'GET' && u.pathname === '/') {
    res.writeHead(200, { 'Content-Type': 'text/html; charset=utf-8' });
    return res.end(PAGE);
  }
  if (req.method === 'POST' && u.pathname === '/api/timing') {
    const b = await readBody(req);
    timingLog.push({ step: b.step, at: new Date().toISOString() });
    return jws(res, 200, { ok: true, log: timingLog });
  }
  if (req.method === 'GET' && u.pathname === '/api/timing') return jws(res, 200, timingLog);
  if (req.method === 'POST' && u.pathname === '/api/submit') {
    const b = await readBody(req);
    const need = b.vtype === 'contractor';
    const taxOk = /^VEN-\d{7}$/.test(String(b.taxid || ''));
    const licOk = !need || /^LIC-\d{6}$/.test(String(b.license || ''));
    const accepted = taxOk && licOk && String(b.company || '').length > 2;
    const rec = { accepted, values: b, at: new Date().toISOString() };
    if (accepted) rec.ref = 'VG-' + String(submissions.filter(s => s.accepted).length + 1).padStart(4, '0');
    submissions.push(rec);
    return jws(res, 200, rec);
  }
  if (req.method === 'GET' && u.pathname === '/api/submissions') return jws(res, 200, submissions);
  jws(res, 404, { error: 'not found' });
}).listen(PORT, '127.0.0.1', () => console.log('VendorGate on http://127.0.0.1:' + PORT + '/'));
