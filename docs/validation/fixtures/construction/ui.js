'use strict';
/**
 * SiteBuild server-rendered UI. Plain form POST + redirect (PRG) — no client-side
 * JavaScript is required for any human interaction path. Tables/forms are computed
 * into local variables first, then composed into one page template.
 */

const W = require('../_lib/web');
const R = require('../_lib/runtime');
const esc = W.esc;

function userName(state, id) {
  const u = R.findUserById(state, id);
  return u ? u.name : id;
}
function findVendor(s, id) { return (s.contractors || []).find((c) => c.id === id); }
function vendorName(s, id) {
  const c = findVendor(s, id);
  return c ? c.company : id;
}
function table(headers, arr, rowFn) {
  return W.tbl(headers, arr.map(rowFn).join(''));
}

// ---------------------------------------------------------------- pages

function dashboard(ctx) {
  const s = ctx.state;
  const openPRs = s.purchaseRequests.filter((r) => r.status === 'submitted').length;
  const openInc = s.safetyIncidents.filter((i) => i.status !== 'closed').length;
  const myNotifs = ctx.user ? s.notifications.filter((n) => n.userId === ctx.user.id) : [];
  const failedNotifs = myNotifs.filter((n) => n.status === 'failed').length;
  const recent = s.events.slice(-8).reverse();
  const projectsTable = table(['Code', 'Project', 'Phase', 'Progress', 'Spend / budget', 'Due'], s.projects, (p) => W.tr([
    `<code>${esc(p.code)}</code>`, `<a href="/projects/${esc(p.id)}">${esc(p.name)}</a>`,
    esc(p.phase), W.progress(p.percentComplete) + ` <span class="muted">${p.percentComplete}%</span>`,
    `${W.money(p.spentUsd)} / ${W.money(p.budgetUsd)}`, esc(p.dueDate),
  ]));
  const eventsTable = W.tbl(['Time', 'Type', 'Summary'], recent.map((e) => W.tr([
    `<span class="mono">${esc(e.ts)}</span>`, `<code>${esc(e.type)}</code>`, esc(e.summary),
  ])).join('') + `<tr><td colspan="3"><a href="/events">Full event feed</a> · machine: <code>GET /api/events</code></td></tr>`);
  const signInHint = ctx.user ? '' :
    '<div class="flash flash-warn" role="alert">You are browsing the public view. <a href="/login">Sign in</a> as a seeded persona to operate the app (personas are listed on the login page).</div>';
  return W.page(ctx.app.config, {
    title: 'Dashboard', user: ctx.user, query: ctx.query,
    body: `
<h1>Construction operations dashboard</h1>
${signInHint}
<div class="grid grid3">
${W.kpi('Active projects', s.projects.length, 'seeded')}
${W.kpi('Purchase requests awaiting decision', openPRs, 'procurement')}
${W.kpi('Open safety incidents', openInc, 'safety')}
${W.kpi('Your notifications', myNotifs.length, failedNotifs + ' failed')}
</div>
${W.card('Projects', projectsTable)}
${W.card('Recent activity (event feed)', eventsTable)}
<p class="muted">Docs: <a href="/failures">failure switches</a> · repository: <code>docs/validation/fixtures/construction/</code> (FAILURES.md + catalog in fixtures/README.md).</p>`,
  });
}

function projects(ctx) {
  const s = ctx.state;
  const t = table(['Code', 'Project', 'Site', 'Phase', 'Progress', 'Manager', 'Contractors'], s.projects, (p) => W.tr([
    `<code>${esc(p.code)}</code>`, `<a href="/projects/${esc(p.id)}">${esc(p.name)}</a>`, esc(p.site),
    esc(p.phase), W.progress(p.percentComplete) + ` <span class="muted">${p.percentComplete}%</span>`,
    esc(userName(s, p.managerId)), p.contractorIds.map((c) => esc(vendorName(s, c))).join(', '),
  ]));
  return W.page(ctx.app.config, {
    title: 'Projects', user: ctx.user, query: ctx.query,
    body: `<h1>Projects</h1>${W.card('All projects', t)}`,
  });
}

function projectPage(ctx) {
  const s = ctx.state;
  const detail = require('./ops').projectDetail(s, ctx.params.id);
  if (!detail) throw new R.AppError(404, 'project_not_found', `Project "${ctx.params.id}" does not exist.`);
  const p = detail.project;
  const canReport = ctx.user && ctx.user.permissions.includes('progress:create');
  const photoOptions = detail.photos.map((a) => [a.id, `${a.id} — ${a.name}`]);
  const taskOptions = detail.tasks.filter((t) => t.status !== 'done').map((t) => [t.id, `${t.id} — ${t.title}`]);
  const overview = W.dl([
    ['Site', esc(p.site)], ['Phase', esc(p.phase)], ['Progress', W.progress(p.percentComplete) + ` ${p.percentComplete}%`],
    ['Budget', `${W.money(p.spentUsd)} of ${W.money(p.budgetUsd)}`], ['Manager', esc(userName(s, p.managerId))],
    ['Contractors', p.contractorIds.map((c) => `${esc(vendorName(s, c))} <span class="muted">until ${esc((findVendor(s, c) || {}).contractValidUntil || '?')}</span>`).join('<br>')],
    ['Due', esc(p.dueDate)],
  ]);
  const tasksTable = table(['Task', 'Trade', 'Status', 'Assignee'], detail.tasks, (t) => W.tr([
    `<code>${esc(t.id)}</code> ${esc(t.title)}`, esc(t.trade), W.statusPill(t.status),
    esc(t.assigneeId ? userName(s, t.assigneeId) : '—'),
  ]));
  const photosTable = detail.photos.length ? table(['Asset', 'Name', 'Caption', 'Size'], detail.photos, (a) => W.tr([
    `<code>${esc(a.id)}</code>`, esc(a.name), esc(a.caption), `${a.sizeKb} kb`,
  ])) : '<p class="muted">No photos yet.</p>';
  const reportForm = canReport ? W.card('Submit daily progress report (foreman)', W.frm('/ui/progress', [
    W.hidden('projectId', p.id),
    W.fld('Report date', W.txt('reportDate', s.meta.today, { type: 'date', required: true })),
    taskOptions.length ? W.fld('Tasks completed', W.sel('taskIds', taskOptions, { multi: true, size: Math.min(6, taskOptions.length) }), 'Ctrl/Cmd-click to multi-select') : '',
    W.fld('Percent complete', W.txt('percentComplete', p.percentComplete, { type: 'number', min: 0, max: 100 })),
    photoOptions.length ? W.fld('Photos attached', W.sel('photoIds', photoOptions, { multi: true, size: Math.min(4, photoOptions.length) }), 'Optional; pick site photos') : '',
    W.fld('Notes', W.ta('notes', '')),
  ], { submit: 'Submit daily report', note: 'Creates the report, marks tasks done, updates project %, notifies the PM. The API twin POST /api/progress accepts the same fields plus ?failure= switches.' })) : '';
  const uploadForm = canReport ? W.card('Upload site photo', W.frm('/ui/assets/upload', [
    W.hidden('projectId', p.id),
    W.fld('File name', W.txt('name', 'level13-formwork.jpg', { required: true })),
    W.fld('Caption', W.txt('caption', '')),
    W.fld('Size (kb)', W.txt('sizeKb', '1500', { type: 'number', min: 1 })),
  ], { submit: 'Upload photo', note: 'Simulated upload — records the asset and emits asset.uploaded.' })) : '';
  const reportsTable = detail.reports.length ? table(['Report', 'Date', 'By', '%', 'Tasks', 'Photos', 'Notes'], detail.reports.slice().reverse(), (r) => W.tr([
    `<code>${esc(r.id)}</code>`, esc(r.reportDate), esc(userName(s, r.authorId)), `${r.percentComplete}%`,
    esc(r.taskIds.join(', ') || '—'), esc(r.photoIds.join(', ') || '—'), esc(r.notes),
  ])) : '<p class="muted">No reports yet.</p>';
  const prList = detail.purchaseRequests.length
    ? `<ul>${detail.purchaseRequests.map((r) => `<li><code>${esc(r.id)}</code> ${esc(r.item)} — ${W.statusPill(r.status)} <span class="muted">${W.money(r.totalUsd)} · ${esc(vendorName(s, r.vendorId))} · contract until ${esc((findVendor(s, r.vendorId) || {}).contractValidUntil || '?')}</span></li>`).join('')}</ul>`
    : '<p class="muted">No purchase requests.</p>';
  const incidentsTable = detail.incidents.length ? table(['ID', 'Severity', 'Category', 'Status', 'Description'], detail.incidents, (i) => W.tr([
    `<code>${esc(i.id)}</code>`, W.statusPill(i.severity), esc(i.category), W.statusPill(i.status), esc(i.description),
  ])) : '<p class="muted">No incidents on this project.</p>';
  const docsTable = detail.documents.length ? table(['Document', 'Version', 'Owner', 'Asset'], detail.documents, (d) => W.tr([
    esc(d.name), 'v' + d.version, esc(userName(s, d.owner)), d.assetId ? `<code>${esc(d.assetId)}</code>` : '<span class="muted">not attached</span>',
  ])) : '<p class="muted">No documents.</p>';
  return W.page(ctx.app.config, {
    title: p.name, user: ctx.user, query: ctx.query,
    body: `
<h1>${esc(p.name)} <span class="muted">${esc(p.code)}</span></h1>
${W.card('Overview', overview)}
${W.card('Tasks', tasksTable)}
${W.card('Photos on file', photosTable)}
${reportForm}
${uploadForm}
${W.card('Progress reports', reportsTable)}
${W.card('Procurement', prList)}
${W.card('Safety incidents', incidentsTable)}
${W.card('Documents', docsTable)}`,
  });
}

function procurement(ctx) {
  const s = ctx.state;
  const canApprove = ctx.user && ctx.user.permissions.includes('po:approve');
  const canInvoice = ctx.user && ctx.user.permissions.includes('invoice:approve');
  const prRows = s.purchaseRequests.map((r) => W.tr([
    `<code>${esc(r.id)}</code>`, esc(r.item), `${r.qty} ${esc(r.unit)}`, W.money(r.totalUsd),
    esc(vendorName(s, r.vendorId)), `<span class="muted">until ${esc((findVendor(s, r.vendorId) || {}).contractValidUntil || '?')}</span>`,
    `<code>${esc(r.projectId)}</code>`, W.statusPill(r.status),
    canApprove && (r.status === 'submitted' || r.status === 'screening')
      ? W.frm('/ui/procurement/decide', [
          W.hidden('requestId', r.id),
          W.fld('Decision', W.sel('decision', [['approve', 'Approve (issue PO)'], ['reject', 'Reject']])),
          W.fld('Note', W.txt('note', '')),
        ], { submit: 'Decide' })
      : (r.approvals.map((a) => `${esc(a.decision)} by ${esc(userName(s, a.byId))}`).join('; ') || '—'),
  ])).join('');
  const poRows = s.purchaseOrders.map((o) => W.tr([
    `<code>${esc(o.id)}</code>`, esc(vendorName(s, o.vendorId)), W.money(o.totalUsd), W.statusPill(o.status), esc(userName(s, o.issuedById)),
  ])).join('');
  const invRows = s.invoices.map((v) => W.tr([
    `<code>${esc(v.id)}</code>`, esc(vendorName(s, v.vendorId)), `<code>${esc(v.poNumber || '—')}</code>`,
    W.money(v.amountUsd), esc(v.dueDate || '—'), W.statusPill(v.status),
    canInvoice && v.status === 'received'
      ? W.frm('/ui/invoices/approve', [W.hidden('invoiceId', v.id)], { submit: 'Approve for payment' })
      : (v.approvedById ? esc(userName(s, v.approvedById)) : '—'),
  ])).join('');
  return W.page(ctx.app.config, {
    title: 'Procurement & finance', user: ctx.user, query: ctx.query,
    body: `
<h1>Procurement &amp; finance</h1>
${W.card('Purchase requests' + (canApprove ? ' — decide as project manager' : ''), W.tbl(
  ['PR', 'Item', 'Qty', 'Total', 'Vendor', 'Contract', 'Project', 'Status', canApprove ? 'Decision' : 'History'], prRows))}
${W.card('Purchase orders', W.tbl(['PO', 'Vendor', 'Total', 'Status', 'Issued by'], poRows))}
${W.card('Invoices' + (canInvoice ? ' — approve as finance' : ''), W.tbl(
  ['Invoice', 'Vendor', 'PO', 'Amount', 'Due', 'Status', canInvoice ? 'Action' : 'Approved by'], invRows))}`,
  });
}

function safety(ctx) {
  const s = ctx.state;
  const canReport = ctx.user && ctx.user.permissions.includes('safety:report');
  const canClose = ctx.user && ctx.user.permissions.includes('safety:close');
  const rows = s.safetyIncidents.slice().reverse().map((i) => W.tr([
    `<code>${esc(i.id)}</code>`, `<code>${esc(i.projectId)}</code>`, W.statusPill(i.severity), esc(i.category),
    W.statusPill(i.status), esc(i.description), esc(userName(s, i.reportedById)),
    canClose && i.status !== 'closed'
      ? W.frm('/ui/safety/close', [W.hidden('incidentId', i.id), W.fld('Resolution', W.txt('resolution', ''))], { submit: 'Close incident' })
      : esc(((i.actions[i.actions.length - 1] || {}).text) || '—'),
  ])).join('');
  const reportForm = canReport ? W.card('Report a safety incident', W.frm('/ui/safety/report', [
    W.fld('Project', W.sel('projectId', s.projects.map((p) => [p.id, `${p.code} — ${p.name}`]))),
    W.fld('Severity', W.sel('severity', [['minor', 'Minor'], ['moderate', 'Moderate'], ['major', 'Major']])),
    W.fld('Category', W.sel('category', ['near-miss', 'utility-strike', 'fall-protection', 'equipment', 'environmental', 'other'])),
    W.fld('Description', W.ta('description', 'What happened, where, who was involved.')),
  ], { submit: 'Report incident', note: 'Records the incident and notifies the safety officer.' })) : '';
  return W.page(ctx.app.config, {
    title: 'Safety', user: ctx.user, query: ctx.query,
    body: `
<h1>Safety incidents</h1>
${W.card('All incidents', W.tbl(['ID', 'Project', 'Severity', 'Category', 'Status', 'Description', 'Reported by', canClose ? 'Resolution / action' : 'Last action'], rows))}
${reportForm}`,
  });
}

function notifications(ctx) {
  const s = ctx.state;
  R.requireUser(ctx);
  const mine = s.notifications.filter((n) => n.userId === ctx.user.id).slice().reverse();
  const rows = mine.map((n) => W.tr([
    `<code>${esc(n.id)}</code>`, esc(n.type), `<b>${esc(n.subject)}</b>`, esc(n.text),
    W.statusPill(n.status), `<span class="mono">${esc(n.createdAt)}</span>`,
  ])).join('');
  return W.page(ctx.app.config, {
    title: 'Notifications', user: ctx.user, query: ctx.query,
    body: `<h1>Your notifications</h1>
<p class="muted">In-app notification inbox. Failed deliveries stay visible with a <span class="pill p-err">failed</span> badge — that is the observable symptom of the <code>notification_failure</code> switch.</p>
${W.card('Inbox', mine.length ? W.tbl(['ID', 'Type', 'Subject', 'Text', 'Status', 'At'], rows) : '<p class="muted">No notifications.</p>')}`,
  });
}

module.exports = { dashboard, projects, project: projectPage, procurement, safety, notifications };
