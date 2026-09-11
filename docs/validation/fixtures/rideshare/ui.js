'use strict';
/**
 * RidePilot server-rendered UI — dashboard, onboarding, public intake, support,
 * ops (surge + broadcasts), map, payments, incidents, drivers.
 */

const W = require('../_lib/web');
const R = require('../_lib/runtime');
const esc = W.esc;

function userName(s, id) {
  const u = R.findUserById(s, id);
  return u ? u.name : (id || '—');
}
function regionName(s, id) {
  const r = (s.regions || []).find((x) => x.id === id);
  return r ? r.name : id;
}
function table(headers, arr, rowFn) { return W.tbl(headers, arr.map(rowFn).join('')); }
function can(ctx, perm) { return ctx.user && ctx.user.permissions.includes(perm); }
function money(cents) { return '$' + (cents / 100).toFixed(2); }

function dashboard(ctx) {
  const s = ctx.state;
  const pendingApps = s.driverApplications.filter((a) => ['submitted', 'screening'].includes(a.status));
  const openTickets = s.supportTickets.filter((t) => t.status !== 'resolved');
  const regionsTable = table(['Region', 'Code', 'Surge', 'Active drivers', 'Demand', 'Status'], s.regions, (r) => W.tr([
    `<b>${esc(r.name)}</b>`, `<code>${esc(r.code)}</code>`, `${r.surgeLevel.toFixed(1)}×`, r.activeDrivers,
    W.progress(r.demandIndex) + ` <span class="muted">${r.demandIndex}</span>`, W.statusPill(r.status),
  ]));
  const recent = s.events.slice(-8).reverse();
  return W.page(ctx.app.config, {
    title: 'Dashboard', user: ctx.user, query: ctx.query,
    body: `
<h1>RidePilot — rideshare operations</h1>
${ctx.user ? '' : '<div class="flash flash-warn" role="alert">Public view. <a href="/login">Sign in</a> as a seeded persona (ops director, ops manager, support agent, finance) to operate. Drivers apply via the <a href="/intake">public intake page</a>.</div>'}
<div class="grid grid3">
${W.kpi('Active regions', s.regions.length, s.regions.filter((r) => r.status === 'surge').length + ' in surge')}
${W.kpi('Applications awaiting screening', pendingApps.length, 'onboarding')}
${W.kpi('Open support tickets', openTickets.length, 'support')}
${W.kpi('Pending payouts', s.payments.filter((p) => p.status === 'pending').length, 'finance')}
</div>
${W.card('Regions & surge', regionsTable)}
${W.card('Recent activity', table(['Time', 'Type', 'Summary'], recent, (e) => W.tr([
  `<span class="mono">${esc(e.ts)}</span>`, `<code>${esc(e.type)}</code>`, esc(e.summary),
])))}
<p class="muted"><a href="/map">Live region map</a> · <a href="/failures">failure switches</a> · repository: <code>docs/validation/fixtures/rideshare/</code>.</p>`,
  });
}

function onboarding(ctx) {
  const s = ctx.state;
  const canScreen = can(ctx, 'driver:screen');
  const canActivate = can(ctx, 'driver:activate');
  const rows = s.driverApplications.map((a) => W.tr([
    `<code>${esc(a.id)}</code>`, esc(a.name), `<code>${esc(a.vehicle.plate)}</code>`, esc(regionName(s, a.regionId)),
    a.docs.map((d) => `${esc(d.kind)}:${W.statusPill(d.status)}`).join(' '),
    W.statusPill(a.backgroundCheck), W.statusPill(a.status),
    `<span class="muted">permit until ${esc(a.preIssuedPermitValidUntil)}</span>`,
    canScreen && ['submitted', 'screening'].includes(a.status)
      ? W.frm('/ui/applications/screen', [
          W.hidden('applicationId', a.id),
          W.fld('Decision', W.sel('decision', [['approve', 'Approve'], ['reject', 'Reject']])),
          W.fld('Notes', W.txt('notes', '')),
        ], { submit: 'Decide' })
      : '—',
    canActivate && a.status === 'approved'
      ? W.frm('/ui/drivers/activate', [W.hidden('applicationId', a.id)], { submit: 'Activate driver' })
      : '—',
  ])).join('');
  const driversTable = table(['Driver', 'Region', 'Status', 'Rating', 'Trips', 'Earnings', 'Permit valid'], s.drivers, (d) => W.tr([
    `<code>${esc(d.id)}</code> ${esc(d.name)}`, esc(regionName(s, d.regionId)), W.statusPill(d.status),
    d.rating ? d.rating.toFixed(2) : '—', d.tripsCompleted, money(d.earningsCents), esc(d.permitValidUntil),
  ]));
  return W.page(ctx.app.config, {
    title: 'Driver onboarding', user: ctx.user, query: ctx.query,
    body: `<h1>Driver onboarding</h1>
${W.card('Applications', W.tbl(['ID', 'Name', 'Plate', 'Region', 'Documents', 'Background', 'Status', 'Permit', 'Screen', 'Activate'], rows))}
${W.card('Active drivers', driversTable)}
<p class="muted">Public application intake: <a href="/intake">/intake</a> (no sign-in required — like the real apply page).</p>`,
  });
}

function intake(ctx) {
  const s = ctx.state;
  return W.page(ctx.app.config, {
    title: 'Apply to drive', user: ctx.user, query: ctx.query,
    body: `<h1>Apply to drive with RidePilot</h1>
${W.card('Driver application (public intake)', W.frm('/ui/intake/apply', [
  W.fld('Full name', W.txt('name', '', { required: true })),
  W.fld('Phone (last 4 shown only)', W.txt('phone', '', { required: true })),
  W.fld('Email', W.txt('email', '')),
  W.fld('Region', W.sel('regionId', s.regions.map((r) => [r.id, `${r.name} (${r.code})`]))),
  W.fld('Vehicle: make', W.txt('make', 'Toyota')),
  W.fld('Vehicle: model', W.txt('model', 'Corolla')),
  W.fld('Vehicle: year', W.txt('year', '2021', { type: 'number', min: 2005, max: 2027 })),
  W.fld('Vehicle: plate', W.txt('plate', '', { required: true })),
  W.fld('Do you have your license document ready?', W.sel('licenseDoc', [['yes', 'Yes'], ['no', 'No — not yet']])),
  W.fld('Do you have your insurance document ready?', W.sel('insuranceDoc', [['yes', 'Yes'], ['no', 'No — not yet']])),
], { submit: 'Submit application', note: 'This public form is the natural duplicate_event case: submitting the same plate twice is rejected.' }))}
<p class="muted">This is a fake rideshare company used for workflow validation. No real data is collected or stored.</p>`,
  });
}

function support(ctx) {
  const s = ctx.state;
  const canRespond = can(ctx, 'ticket:respond');
  const canEscalate = can(ctx, 'ticket:escalate');
  const canResolve = can(ctx, 'ticket:resolve');
  const cards = s.supportTickets.map((t) => W.card(`Ticket ${esc(t.id)} — ${esc(t.subject)} ${W.statusPill(t.status)}`, `
${W.dl([['Opened by', esc(t.openedBy)], ['Category', esc(t.category)], ['Priority', W.statusPill(t.priority)], ['Assignee', esc(t.assigneeId ? userName(s, t.assigneeId) : '—')]])}
${table(['From', 'Role', 'At', 'Message'], t.messages, (m) => W.tr([esc(m.from), W.statusPill(m.role === 'agent' ? 'info' : 'neutral') + ' ' + esc(m.role), `<span class="mono">${esc(m.at)}</span>`, esc(m.text)]))}
${canRespond && t.status !== 'resolved' ? W.frm('/ui/tickets/respond', [
  W.hidden('ticketId', t.id), W.fld('Reply', W.ta('message', '')),
], { submit: 'Send reply' }) : ''}
${canEscalate && t.status === 'open' ? W.frm('/ui/tickets/escalate', [
  W.hidden('ticketId', t.id), W.fld('Escalation reason', W.txt('reason', 'Needs ops attention')),
], { submit: 'Escalate to ops', sec: true }) : ''}
${canResolve && t.status !== 'resolved' ? W.frm('/ui/tickets/resolve', [
  W.hidden('ticketId', t.id), W.fld('Resolution', W.txt('resolution', 'Handled.')),
], { submit: 'Resolve', sec: true }) : ''}`)).join('');
  return W.page(ctx.app.config, {
    title: 'Support', user: ctx.user, query: ctx.query,
    body: `<h1>Driver &amp; passenger support</h1>${cards}`,
  });
}

function ops(ctx) {
  const s = ctx.state;
  const canSurge = can(ctx, 'surge:adjust');
  const surgeForm = canSurge ? W.card('Adjust surge (ops decision workflow)', W.frm('/ui/regions/surge', [
    W.fld('Region', W.sel('regionId', s.regions.map((r) => [r.id, `${r.name} (${r.surgeLevel.toFixed(1)}×)`]))),
    W.fld('New surge level (1.0 – 3.0)', W.txt('level', '1.5', { type: 'number', min: 1.0, max: 3.0, step: 0.1 })),
  ], { submit: 'Apply surge', note: 'Broadcasts the change to region drivers (notification_failure makes the broadcast fail).' })) : '';
  const bTable = table(['Broadcast', 'Region', 'Channel', 'Message', 'Status', 'Sent by', 'At'], s.broadcasts.slice().reverse(), (b) => W.tr([
    `<code>${esc(b.id)}</code>`, esc(regionName(s, b.regionId)), esc(b.channel), esc(b.message),
    W.statusPill(b.status), esc(userName(s, b.sentById)), `<span class="mono">${esc(b.at)}</span>`,
  ]));
  return W.page(ctx.app.config, {
    title: 'Operations', user: ctx.user, query: ctx.query,
    body: `<h1>Operations dashboard</h1>
${surgeForm}
${W.card('Driver broadcasts (communications)', bTable)}
<p class="muted"><a href="/map">Live region map →</a></p>`,
  });
}

function map(ctx) {
  const s = ctx.state;
  const CELL = 100, PAD = 4;
  const rects = s.regions.map((r) => {
    const x = r.map.x * CELL + PAD, y = r.map.y * CELL + PAD, w = r.map.w * CELL - 2 * PAD, h = r.map.h * CELL - 2 * PAD;
    const dots = [];
    const n = Math.min(r.activeDrivers, 12);
    for (let i = 0; i < n; i++) {
      const dx = x + 14 + ((i * 37) % Math.max(1, w - 28));
      const dy = y + 30 + ((i * 53) % Math.max(1, h - 44));
      dots.push(`<circle cx="${dx}" cy="${dy}" r="4" fill="#0f766e" opacity="0.75"><title>driver</title></circle>`);
    }
    const color = r.status === 'surge' ? '#f59e0b' : '#0f766e';
    return `<g><rect x="${x}" y="${y}" width="${w}" height="${h}" rx="8" fill="${color}" fill-opacity="0.12" stroke="${color}" stroke-width="1.5"/>
<text x="${x + 10}" y="${y + 20}" font-size="13" font-weight="700" fill="#1c2430">${esc(r.code)} — ${esc(r.name)}</text>
<text x="${x + 10}" y="${y + 36}" font-size="11" fill="#475569">surge ${r.surgeLevel.toFixed(1)}× · ${r.activeDrivers} drivers · demand ${r.demandIndex}</text>
${dots.join('')}</g>`;
  }).join('');
  return W.page(ctx.app.config, {
    title: 'Region map', user: ctx.user, query: ctx.query,
    body: `<h1>Live region map</h1>
${W.card('Regions, surge state and driver presence', `
<svg class="map" viewBox="0 0 620 412" role="img" aria-label="Region map with driver presence">${rects}</svg>
<p class="muted">Driver dots are deterministic per region driver count (capped at 12 for readability). Amber regions are in surge.</p>`)}
<p class="muted"><a href="/ops">← Operations dashboard</a></p>`,
  });
}

function payments(ctx) {
  const s = ctx.state;
  const canApprove = can(ctx, 'payout:approve');
  const rows = s.payments.map((p) => W.tr([
    `<code>${esc(p.id)}</code>`, driverCell(s, p.driverId), esc(p.payPeriod), p.tripsCount,
    money(p.grossCents), money(p.commissionCents), `<b>${money(p.netCents)}</b>`, W.statusPill(p.status),
    canApprove && p.status === 'pending' ? W.frm('/ui/payments/approve', [W.hidden('paymentId', p.id)], { submit: 'Approve payout' }) : '—',
  ])).join('');
  return W.page(ctx.app.config, {
    title: 'Payments & earnings', user: ctx.user, query: ctx.query,
    body: `<h1>Driver payouts</h1>
<p class="muted">Approving a payout for a driver whose regional permit has expired returns <code>stale_entitlement</code> (seed includes such a driver: Lena Marsh, permit ended 2026-07-01).</p>
${W.card('Payout batches', W.tbl(['Payment', 'Driver', 'Period', 'Trips', 'Gross', 'Commission', 'Net', 'Status', 'Action'], rows))}`,
  });
}
function driverCell(s, id) {
  const d = (s.drivers || []).find((x) => x.id === id);
  return d ? `<code>${esc(d.id)}</code> ${esc(d.name)} <span class="muted">permit ${esc(d.permitValidUntil)}</span>` : id;
}

function incidents(ctx) {
  const s = ctx.state;
  const canReport = can(ctx, 'incident:report');
  const canResolve = can(ctx, 'incident:resolve');
  const rows = s.incidents.map((i) => W.tr([
    `<code>${esc(i.id)}</code>`, i.tripId ? `<code>${esc(i.tripId)}</code>` : '—', driverCell(s, i.driverId),
    esc(i.category), W.statusPill(i.severity), W.statusPill(i.status), esc(i.description),
    canResolve && i.status !== 'resolved' ? W.frm('/ui/incidents/resolve', [
      W.hidden('incidentId', i.id), W.fld('Resolution', W.txt('resolution', 'Reviewed and closed.')),
    ], { submit: 'Resolve' }) : '—',
  ])).join('');
  const reportForm = canReport ? W.card('Report an incident', W.frm('/ui/incidents/report', [
    W.fld('Driver', W.sel('driverId', [['', '— no driver —']].concat(s.drivers.map((d) => [d.id, `${d.id} — ${d.name}`])))),
    W.fld('Trip (optional)', W.txt('tripId', '')),
    W.fld('Category', W.sel('category', ['unsafe-driving', 'accident', 'harassment', 'vehicle-issue', 'other'])),
    W.fld('Severity', W.sel('severity', [['low', 'Low'], ['medium', 'Medium'], ['high', 'High']])),
    W.fld('Description', W.ta('description', 'What happened?')),
  ], { submit: 'Report incident' })) : '';
  return W.page(ctx.app.config, {
    title: 'Incidents', user: ctx.user, query: ctx.query,
    body: `<h1>Incident management</h1>
${W.card('All incidents', W.tbl(['ID', 'Trip', 'Driver', 'Category', 'Severity', 'Status', 'Description', 'Action'], rows))}
${reportForm}`,
  });
}

function notifications(ctx) {
  const s = ctx.state;
  R.requireUser(ctx);
  const mine = s.notifications.filter((n) => n.userId === ctx.user.id).slice().reverse();
  const rows = mine.map((n) => W.tr([
    `<code>${esc(n.id)}</code>`, esc(n.type), `<b>${esc(n.subject)}</b>`, esc(n.text), W.statusPill(n.status),
  ])).join('');
  return W.page(ctx.app.config, {
    title: 'Notifications', user: ctx.user, query: ctx.query,
    body: `<h1>Your notifications</h1>${W.card('Inbox', mine.length ? W.tbl(['ID', 'Type', 'Subject', 'Text', 'Status'], rows) : '<p class="muted">No notifications.</p>')}`,
  });
}

module.exports = { dashboard, onboarding, intake, support, ops, map, payments, incidents, notifications };
