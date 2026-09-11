'use strict';
/**
 * VWO-002 fixture web toolkit — page shell, shared CSS, shared pages (login, events,
 * failures, error) and small HTML composition helpers used by every family's ui.js.
 * Server-rendered HTML only; no client framework, no external assets, no JavaScript
 * required for the core human interaction paths (plain form POST + redirect).
 */

function esc(s) {
  return String(s === null || s === undefined ? '' : s)
    .replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;').replace(/'/g, '&#39;');
}

const CSS = `
:root{--accent:#0f766e;--accent-soft:#e6f2f0;--ink:#1c2430;--muted:#5b6675;--line:#e3e6ea;--bg:#f5f6f8;--card:#ffffff}
*{box-sizing:border-box}
body{margin:0;font:15px/1.55 system-ui,-apple-system,"Segoe UI",Roboto,sans-serif;color:var(--ink);background:var(--bg);min-height:100vh;display:flex;flex-direction:column}
header.bar{background:var(--ink);color:#fff;padding:.65rem 1rem}
.bar-in{max-width:1100px;margin:0 auto;display:flex;flex-wrap:wrap;gap:.5rem 1.25rem;align-items:baseline}
.bar-in .brand{font-weight:700;font-size:1.05rem;color:#fff;text-decoration:none}
.bar-in .brand .fam{color:var(--accent-soft);font-weight:400;font-size:.78rem;margin-left:.4rem}
nav.top{display:flex;flex-wrap:wrap;gap:.1rem .9rem;align-items:baseline}
nav.top a{color:#cfd6df;text-decoration:none;font-size:.9rem;padding:.15rem 0}
nav.top a:hover{color:#fff}
.who{margin-left:auto;font-size:.82rem;color:#cfd6df}
.who a{color:#fff}
main{flex:1;width:100%;max-width:1100px;margin:0 auto;padding:1.1rem 1rem 2.5rem}
footer.foot{background:var(--ink);color:#9aa6b3;text-align:center;font-size:.75rem;padding:.9rem 1rem}
footer.foot b{color:#dfe5ea;font-weight:600}
h1{font-size:1.45rem;margin:.2rem 0 .8rem}h2{font-size:1.08rem;margin:1.4rem 0 .55rem}h3{font-size:.95rem;margin:1rem 0 .4rem}
.card{background:var(--card);border:1px solid var(--line);border-radius:10px;padding:1rem 1.2rem;margin-bottom:1rem}
.card h2:first-child,h1+h2{margin-top:0}
.muted{color:var(--muted);font-size:.85rem}
.pill{display:inline-block;border-radius:999px;padding:.05rem .6rem;font-size:.72rem;font-weight:600;white-space:nowrap}
.p-ok{background:#e2f3e6;color:#166534}.p-warn{background:#fdf0d5;color:#92400e}.p-err{background:#fbe3e3;color:#991b1b}
.p-neutral{background:#eceff3;color:#45505e}.p-info{background:var(--accent-soft);color:var(--ink)}
table.tbl{width:100%;border-collapse:collapse;font-size:.88rem}
.tbl th{text-align:left;color:var(--muted);font-weight:600;font-size:.76rem;text-transform:uppercase;letter-spacing:.03em;padding:.35rem .5rem;border-bottom:2px solid var(--line)}
.tbl td{padding:.42rem .5rem;border-bottom:1px solid var(--line);vertical-align:top}
.tbl tr:hover td{background:#fafbfc}
.tblwrap{overflow-x:auto}
form.frm{display:grid;gap:.55rem;max-width:560px}
.frm label{font-size:.82rem;font-weight:600;display:block;margin-bottom:-.35rem}
.frm input,.frm select,.frm textarea{width:100%;padding:.45rem .6rem;border:1px solid #c9cfd7;border-radius:8px;font:inherit;background:#fff}
.frm textarea{min-height:4.2rem;resize:vertical}
.frm .hint{font-size:.75rem;color:var(--muted);margin-top:-.4rem}
button.btn,.btn{display:inline-block;border:0;border-radius:8px;padding:.45rem .95rem;font:inherit;font-weight:600;cursor:pointer;background:var(--accent);color:#fff;text-decoration:none}
.btn:hover{filter:brightness(1.08)}
.btn.sec{background:#fff;color:var(--ink);border:1px solid #c9cfd7}
.btn.mini{padding:.2rem .6rem;font-size:.78rem}
.flash{border-radius:8px;padding:.6rem .9rem;margin-bottom:1rem;font-size:.9rem}
.flash-ok{background:#e2f3e6;color:#14532d;border:1px solid #b7dfc2}
.flash-warn{background:#fdf0d5;color:#7c3f0c;border:1px solid #f0d9a8}
.flash-err{background:#fbe3e3;color:#7f1d1d;border:1px solid #f0c4c4}
.grid{display:grid;gap:1rem}
.grid2{grid-template-columns:repeat(auto-fit,minmax(300px,1fr))}
.grid3{grid-template-columns:repeat(auto-fit,minmax(215px,1fr))}
.kpi{font-size:1.7rem;font-weight:700;margin:.1rem 0}
.kpi small{font-size:.78rem;color:var(--muted);font-weight:400}
.bar-track{height:8px;border-radius:6px;background:#e8ebee;overflow:hidden;min-width:90px}
.bar-fill{height:100%;background:var(--accent)}
code,.mono{font-family:ui-monospace,SFMono-Regular,Menlo,monospace;font-size:.82em;background:#eef1f4;border-radius:5px;padding:.06rem .3rem}
a{color:var(--accent)}
.kbd{font-family:ui-monospace,Menlo,monospace;font-size:.78rem;border:1px solid #c9cfd7;border-bottom-width:2px;border-radius:5px;padding:0 .35rem;background:#fff}
details.demo{margin-top:.8rem}
details.demo summary{cursor:pointer;font-size:.85rem;color:var(--muted)}
.demo-note{font-size:.78rem;color:var(--muted);margin:.3rem 0 .5rem}
input[type=checkbox],input[type=radio]{width:auto}
.frm .checks{display:flex;flex-wrap:wrap;gap:.35rem 1rem;font-size:.88rem;font-weight:400}
.frm .checks label{margin:0;font-weight:400;display:inline}
.map{width:100%;height:auto;background:#eef2f4;border-radius:8px}
@media(max-width:720px){.bar-in{gap:.35rem .8rem}.who{margin-left:0;width:100%}main{padding:.8rem .7rem 2rem}.card{padding:.85rem .9rem}}
`;

function page(config, opts) {
  const flash = [];
  const q = opts.query;
  if (q && q.get('ok')) flash.push(`<div class="flash flash-ok" role="status">${esc(q.get('ok'))}</div>`);
  if (q && q.get('warn')) flash.push(`<div class="flash flash-warn" role="alert">${esc(q.get('warn'))}</div>`);
  if (q && q.get('err')) flash.push(`<div class="flash flash-err" role="alert">${esc(q.get('err'))}</div>`);
  const nav = (config.nav || []).map(([href, label]) => `<a href="${esc(href)}">${esc(label)}</a>`).join('');
  const who = opts.user
    ? `<span class="who">${esc(opts.user.name)} · ${esc(opts.user.role)} · <a href="/logout" onclick="return true">sign out</a><form method="post" action="/logout" style="display:none" id="lo"></form></span>`
    : `<span class="who"><a href="/login">sign in</a></span>`;
  return `<!doctype html>
<html lang="en"><head>
<meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1">
<title>${esc(opts.title || config.appTitle)} — ${esc(config.appTitle)}</title>
<style>:root{--accent:${config.accent};--accent-soft:${config.accentSoft}}${CSS}</style>
</head><body>
<header class="bar"><div class="bar-in">
<a class="brand" href="/">${esc(config.appTitle)}<span class="fam">${esc(config.family)} fixture · VWO-002</span></a>
<nav class="top" aria-label="Main">${nav}</nav>${who}
</div></header>
<main>
${flash.join('')}
${opts.body}
</main>
<footer class="foot"><b>${esc(config.appTitle)}</b> is a synthetic validation fixture (work order VWO-002) — all data, users and credentials are fake and reset with <span class="kbd">POST /reset</span>.</footer>
</body></html>`;
}

// ---- composition helpers -------------------------------------------------

function card(title, bodyHtml, extraClass) {
  return `<section class="card ${extraClass || ''}">${title ? `<h2>${esc(title)}</h2>` : ''}${bodyHtml}</section>`;
}
function tbl(headers, rowsHtml) {
  return `<div class="tblwrap"><table class="tbl"><thead><tr>${headers.map((h) => `<th>${esc(h)}</th>`).join('')}</tr></thead><tbody>${rowsHtml || '<tr><td class="muted">No records yet.</td></tr>'}</tbody></table></div>`;
}
function tr(cells) { return `<tr>${cells.map((c) => `<td>${c}</td>`).join('')}</tr>`; }
function pill(cls, label) { return `<span class="pill p-${cls}">${esc(label)}</span>`; }
function statusPill(status) {
  const s = String(status || '');
  const cls = /fail|reject|down|err|expired|revoked|stale|blocked/.test(s) ? 'err'
    : /pend|process|review|screen|submitt|queued|investigat|escalat|changes/i.test(s) ? 'warn'
    : /pass|approv|paid|deliver|publish|sent|succeed|active|online|ready|healthy|posted|closed|resolved|done/i.test(s) ? 'ok' : 'neutral';
  return pill(cls, s);
}
function kpi(label, value, sub) {
  return `<div class="card"><div class="muted">${esc(label)}</div><div class="kpi">${esc(value)} <small>${esc(sub || '')}</small></div></div>`;
}
function progress(pct) {
  const p = Math.max(0, Math.min(100, Number(pct) || 0));
  return `<div class="bar-track" title="${p}%"><div class="bar-fill" style="width:${p}%"></div></div>`;
}
function fld(label, inputHtml, hint) {
  return `<div><label>${esc(label)}</label>${inputHtml}${hint ? `<div class="hint">${esc(hint)}</div>` : ''}</div>`;
}
function txt(name, value, o) {
  const oo = o || {};
  return `<input name="${esc(name)}" value="${esc(value === undefined ? '' : value)}" type="${esc(oo.type || 'text')}"${oo.required ? ' required' : ''}${oo.placeholder ? ` placeholder="${esc(oo.placeholder)}"` : ''}${oo.min !== undefined ? ` min="${esc(oo.min)}"` : ''}${oo.max !== undefined ? ` max="${esc(oo.max)}"` : ''}${oo.step ? ` step="${esc(oo.step)}"` : ''}>`;
}
function ta(name, value, rows) {
  return `<textarea name="${esc(name)}" rows="${rows || 4}">${esc(value === undefined ? '' : value)}</textarea>`;
}
function sel(name, options, o) {
  const oo = o || {};
  const opts = options.map((op) => {
    const [v, l] = Array.isArray(op) ? op : [op, op];
    const selected = oo.value !== undefined && String(oo.value) === String(v) ? ' selected' : '';
    return `<option value="${esc(v)}"${selected}>${esc(l)}</option>`;
  }).join('');
  return `<select name="${esc(name)}"${oo.multi ? ' multiple size="' + (oo.size || 4) + '"' : ''}>${opts}</select>`;
}
function checks(name, options) {
  return `<div class="checks">${options.map(([v, l, checked]) => `<label><input type="checkbox" name="${esc(name)}" value="${esc(v)}"${checked ? ' checked' : ''}> ${esc(l)}</label>`).join('')}</div>`;
}
function hidden(name, value) { return `<input type="hidden" name="${esc(name)}" value="${esc(value)}">`; }
function frm(action, fieldsHtml, o) {
  const oo = o || {};
  return `<form class="frm" method="post" action="${esc(action)}">${oo.hidden || ''}${fieldsHtml}
<div><button class="btn${oo.sec ? ' sec' : ''}" type="submit">${esc(oo.submit || 'Submit')}</button>${oo.cancel ? ` <a class="btn sec" href="${esc(oo.cancel)}">Cancel</a>` : ''}</div>
${oo.note ? `<div class="hint">${esc(oo.note)}</div>` : ''}</form>`;
}
function link(href, label, cls) { return `<a class="${cls ? 'btn ' + cls : ''}" href="${esc(href)}">${esc(label)}</a>`; }
function when(cond, html) { return cond ? html : ''; }
function money(n) { return '$' + Number(n || 0).toLocaleString('en-US'); }
function pct(n) { return `${Number(n || 0)}%`; }
function dl(pairs) {
  return `<table class="tbl"><tbody>${pairs.map(([k, v]) => tr([`<span class="muted">${esc(k)}</span>`, v])).join('')}</tbody></table>`;
}

// ---- shared pages --------------------------------------------------------

function loginPage(ctx, config, hints) {
  const next = ctx.query.get('next') || '/';
  const body = `
<div class="grid grid2">
<section class="card"><h2>Sign in</h2>
<form class="frm" method="post" action="/login">
${hidden('next', next)}
${fld('Username', txt('username', '', { required: true, placeholder: 'e.g. ' + (hints[0] ? hints[0].username : 'user') }))}
${fld('Password', txt('password', '', { type: 'password', required: true }))}
<div><button class="btn" type="submit">Sign in</button></div>
</form></section>
<section class="card"><h2>Demo world</h2>
<p class="muted">This is a fake enterprise used by the Codex Universal validation program. No real credentials exist: demo passwords are assembled at runtime from fragments. Pick any persona below.</p>
<details class="demo" open><summary>Show demo personas &amp; passwords</summary>
<p class="demo-note">Persona list (also available at <code>GET /api/demo-hints</code>):</p>
${tbl(['Username', 'Password (fake)', 'Role', 'Name'], hints.map((h) => tr([`<code>${esc(h.username)}</code>`, `<code>${esc(h.password)}</code>`, esc(h.role), esc(h.name)])).join(''))}
</details>
</section></div>`;
  return page(config, { title: 'Sign in', body, user: null, query: ctx.query });
}

function eventsPage(ctx, config, events) {
  const body = `<h1>Event feed</h1>
<p class="muted">Every important operation appends an event. Machine feed: <code>GET /events?format=json</code> or <code>GET /api/events?limit=&amp;since=&amp;type=</code>. Newest first.</p>
${tbl(['Time', 'Type', 'Actor', 'Subject', 'Summary'], events.map((e) => tr([
  `<span class="mono">${esc(e.ts)}</span>`, `<code>${esc(e.type)}</code>`, esc(e.actor), esc(e.subject || ''), esc(e.summary || ''),
])).join(''))}`;
  return page(config, { title: 'Events', body, user: ctx.user, query: ctx.query });
}

function failuresPage(ctx, config) {
  const sw = config.failures;
  const body = `<h1>Failure switches</h1>
<p class="muted">Deterministic, per-request failure simulation. Activate by appending <code class="kbd">?failure=&lt;switch&gt;</code> to any request (forms included) or by sending header <code class="kbd">X-Failure-Switch: &lt;switch&gt;</code>. The switch applies only to the request that carries it. Unknown switch names return HTTP 400 with the known list. Machine-readable: <code>GET /api/failures</code>.</p>
${tbl(['Switch', 'Applies to', 'Behavior', 'Observable symptom'], sw.map((f) => tr([
  `<code>${esc(f.switch)}</code>`, esc(f.applies_to || ''), esc(f.behavior || ''), esc(f.symptom || ''),
])).join(''))}
<p class="muted">This table mirrors <code>docs/validation/fixtures/&lt;family&gt;/FAILURES.md</code> in the repository.</p>`;
  return page(config, { title: 'Failure switches', body, user: ctx.user, query: ctx.query });
}

function errorPage(err, config) {
  const body = `<h1>Something went wrong</h1>
<div class="card"><div class="flash flash-err">${esc(err.message)}</div>
<p>HTTP status <b>${err.status}</b> · error code <code>${esc(err.code)}</code></p>
<p class="muted">If this was an intentional failure switch, the response above is the expected observable symptom. See <a href="/failures">failure switches</a>.</p></div>`;
  return page(config, { title: 'Error ' + err.status, body, user: null, query: null });
}

module.exports = {
  esc, page, card, tbl, tr, pill, statusPill, kpi, progress, fld, txt, ta, sel, checks,
  hidden, frm, link, when, money, pct, dl, loginPage, eventsPage, failuresPage, errorPage,
};
