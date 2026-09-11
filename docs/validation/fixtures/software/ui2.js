'use strict';
/**
 * ForgeOps UI part 2 — deployments, incidents, services, issues, docs, teams, notifications.
 */

const W = require('../_lib/web');
const R = require('../_lib/runtime');
const U = require('./ui');
const esc = W.esc;
const { userName, repoName, teamName, svcName, table, can } = U;

function deployments(ctx) {
  const s = ctx.state;
  const deps = s.deployments.slice().reverse();
  const rows = deps.map((d) => W.tr([
    `<code>${esc(d.id)}</code>`, `<code>${esc(repoName(s, d.repoId))}</code>`,
    d.prId ? `<code>${esc(d.prId)}</code>` : '—', W.statusPill(d.env === 'production' ? 'info' : 'neutral') + ` ${d.env}`,
    `<code>${esc(d.version)}</code>`, `<code>${esc(d.artifactId)}</code>`, W.statusPill(d.status),
    d.promotedById ? esc(userName(s, d.promotedById)) : (d.rollbackOf ? `rollback of <code>${esc(d.rollbackOf)}</code>` : '—'),
    (d.env === 'staging' && d.status === 'succeeded' && can(ctx, 'deploy:promote'))
      ? W.frm('/ui/deployments/promote', [W.hidden('deploymentId', d.id)], { submit: 'Promote to production' }) : '',
    (d.env === 'production' && d.status === 'succeeded' && can(ctx, 'deploy:rollback'))
      ? W.frm('/ui/deployments/rollback', [W.hidden('deploymentId', d.id)], { submit: 'Rollback', sec: true }) : '',
  ])).join('');
  return W.page(ctx.app.config, {
    title: 'Deployments', user: ctx.user, query: ctx.query,
    body: `<h1>Deployment console</h1>
<p class="muted">Staging deployments are created automatically when a PR merges. Promoting to production is a human approval gate (release manager). Also available as the terminal desktop console: <code>node deploy-console.js</code>.</p>
${W.card('All deployments', W.tbl(['Deployment', 'Repo', 'PR', 'Env', 'Version', 'Artifact', 'Status', 'By', 'Promote', 'Rollback'], rows))}`,
  });
}

function incidents(ctx) {
  const s = ctx.state;
  const canOpen = can(ctx, 'incident:open');
  const canResolve = can(ctx, 'incident:resolve');
  const rows = s.incidents.slice().reverse().map((i) => W.tr([
    `<code>${esc(i.id)}</code>`, W.statusPill(i.severity), esc(i.title), `<code>${esc(svcName(s, i.serviceId))}</code>`,
    W.statusPill(i.status), esc(userName(s, i.reporterId)), esc(i.description),
    `<ul>${i.timeline.map((t) => `<li><span class="muted">${esc(t.at)} — ${esc(userName(s, t.byId))}:</span> ${esc(t.text)}</li>`).join('')}</ul>`,
    canResolve && i.status !== 'resolved' ? W.frm('/ui/incidents/resolve', [
      W.hidden('incidentId', i.id), W.fld('Resolution', W.txt('resolution', 'Mitigated and verified.')),
    ], { submit: 'Resolve' }) : '—',
  ])).join('');
  const openForm = canOpen ? W.card('Open an incident', W.frm('/ui/incidents/open', [
    W.fld('Service', W.sel('serviceId', s.services.map((v) => [v.id, `${v.name} (T${v.tier})`]))),
    W.fld('Severity', W.sel('severity', [['sev1', 'SEV1 — outage'], ['sev2', 'SEV2 — major'], ['sev3', 'SEV3 — minor']])),
    W.fld('Title', W.txt('title', '', { required: true })),
    W.fld('Description', W.ta('description', 'Customer impact, blast radius, first suspicions.')),
  ], { submit: 'Open incident', note: 'Marks the service degraded/down and pages on-call.' })) : '';
  return W.page(ctx.app.config, {
    title: 'Incidents', user: ctx.user, query: ctx.query,
    body: `<h1>Incident console</h1>
${W.card('All incidents', W.tbl(['ID', 'Severity', 'Title', 'Service', 'Status', 'Reporter', 'Description', 'Timeline', 'Action'], rows))}
${openForm}`,
  });
}

function services(ctx) {
  const s = ctx.state;
  const t = table(['Service', 'Tier', 'Status', 'Version', 'Uptime 30d', 'p95 latency', 'Owner', 'Depends on'], s.services, (v) => W.tr([
    `<b>${esc(v.name)}</b>`, `T${v.tier}`, W.statusPill(v.status), `<code>${esc(v.version)}</code>`,
    `${v.uptime30d}%`, `${v.latencyMs} ms`, esc(teamName(s, v.ownerTeamId)),
    v.dependsOn.map((d) => `<code>${esc(svcName(s, d))}</code>`).join(', ') || '—',
  ]));
  return W.page(ctx.app.config, {
    title: 'Services', user: ctx.user, query: ctx.query,
    body: `<h1>Service dashboard</h1>${W.card('All services', t)}`,
  });
}

function issues(ctx) {
  const s = ctx.state;
  const canCreate = can(ctx, 'issue:create');
  const canClose = can(ctx, 'issue:close');
  const rows = s.issues.map((i) => W.tr([
    `<code>${esc(i.id)}</code>`, `<code>${esc(repoName(s, i.repoId))}</code>`, esc(i.title),
    W.statusPill(i.type), W.statusPill(i.priority), W.statusPill(i.status),
    i.assigneeId ? esc(userName(s, i.assigneeId)) : '—', i.labels.map((l) => W.pill('info', l)).join(' '),
    canClose && i.status !== 'closed' ? W.frm('/ui/issues/close', [W.hidden('issueId', i.id)], { submit: 'Close', sec: true }) : '—',
  ])).join('');
  const createForm = canCreate ? W.card('File an issue', W.frm('/ui/issues/create', [
    W.fld('Repo', W.sel('repoId', s.repos.map((r) => [r.id, r.name]))),
    W.fld('Title', W.txt('title', '', { required: true })),
    W.fld('Type', W.sel('type', [['bug', 'Bug'], ['feature', 'Feature'], ['task', 'Task']])),
    W.fld('Priority', W.sel('priority', [['p0', 'P0'], ['p1', 'P1'], ['p2', 'P2'], ['p3', 'P3']])),
    W.fld('Assignee', W.sel('assigneeId', [['', '— unassigned —']].concat(s.users.map((u) => [u.id, u.name])))),
    W.fld('Labels (comma-sep)', W.txt('labels', '')),
  ], { submit: 'File issue' })) : '';
  return W.page(ctx.app.config, {
    title: 'Issues', user: ctx.user, query: ctx.query,
    body: `<h1>Issue tracker</h1>
${W.card('All issues', W.tbl(['ID', 'Repo', 'Title', 'Type', 'Priority', 'Status', 'Assignee', 'Labels', 'Action'], rows))}
${createForm}`,
  });
}

function docs(ctx) {
  const s = ctx.state;
  const t = table(['Doc', 'Path', 'Owner team', 'Version'], s.docs, (d) => W.tr([
    `<a href="/docs/${esc(d.id)}"><b>${esc(d.title)}</b></a>`, `<code>${esc(d.path)}</code>`, esc(teamName(s, d.ownerTeamId)), `v${d.version}`,
  ]));
  return W.page(ctx.app.config, {
    title: 'Docs', user: ctx.user, query: ctx.query,
    body: `<h1>Internal documentation</h1>${W.card('All docs', t)}`,
  });
}

function docPage(ctx) {
  const s = ctx.state;
  const doc = s.docs.find((d) => d.id === ctx.params.id);
  if (!doc) throw new R.AppError(404, 'doc_not_found', `Document "${ctx.params.id}" does not exist.`);
  const canEdit = can(ctx, 'doc:write');
  const editForm = canEdit ? W.card('Edit document', W.frm('/ui/docs/update', [
    W.hidden('docId', doc.id),
    W.hidden('expectedVersion', doc.version),
    W.fld('Body (v' + doc.version + ' → v' + (doc.version + 1) + ')', W.ta('body', doc.body, 8)),
  ], { submit: 'Save new version', note: 'Optimistic concurrency: saving a stale version returns data_conflict.' })) : '';
  return W.page(ctx.app.config, {
    title: doc.title, user: ctx.user, query: ctx.query,
    body: `
<h1>${esc(doc.title)} <span class="muted">v${doc.version} · ${esc(doc.path)}</span></h1>
${W.card('Contents', `<p style="white-space:pre-wrap">${esc(doc.body)}</p>`)}
${editForm}`,
  });
}

function teams(ctx) {
  const s = ctx.state;
  const cards = s.teams.map((t) => W.card(`Team: ${esc(t.name)}`, W.dl([
    ['Mission', esc(t.mission)],
    ['Members', t.memberIds.map((m) => esc(userName(s, m))).join(', ')],
    ['Owns repos', t.ownsRepos.map((r) => `<code>${esc(repoName(s, r))}</code>`).join(', ') || '—'],
    ['Owns services', t.ownsServices.map((v) => `<code>${esc(svcName(s, v))}</code>`).join(', ') || '—'],
  ]))).join('');
  const userRows = s.users.map((u) => W.tr([
    esc(u.name), `<code>${esc(u.username)}</code>`, esc(u.role), u.permissions.map((p) => W.pill('neutral', p)).join(' '),
  ])).join('');
  return W.page(ctx.app.config, {
    title: 'Teams & ownership', user: ctx.user, query: ctx.query,
    body: `<h1>Ownership directory</h1>${cards}
${W.card('Users & permissions', W.tbl(['Name', 'Username', 'Role', 'Permissions'], userRows))}`,
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

module.exports = { deployments, incidents, services, issues, docs, doc: docPage, teams, notifications };
