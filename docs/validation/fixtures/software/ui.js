'use strict';
/**
 * ForgeOps UI part 1 — dashboard, repos, pull requests, CI.
 * Precomputed-variable style; plain form POST + redirect.
 */

const W = require('../_lib/web');
const R = require('../_lib/runtime');
const esc = W.esc;

function userName(state, id) {
  const u = R.findUserById(state, id);
  return u ? u.name : id;
}
function repoName(state, id) {
  const r = (state.repos || []).find((x) => x.id === id);
  return r ? r.name : id;
}
function table(headers, arr, rowFn) { return W.tbl(headers, arr.map(rowFn).join('')); }
function can(ctx, perm) { return ctx.user && ctx.user.permissions.includes(perm); }

function dashboard(ctx) {
  const s = ctx.state;
  const openPRs = s.pullRequests.filter((p) => p.status === 'open');
  const openInc = s.incidents.filter((i) => i.status !== 'resolved');
  const servicesTable = table(['Service', 'Tier', 'Status', 'Version', 'Uptime 30d', 'p95', 'Owner'], s.services, (v) => W.tr([
    `<b>${esc(v.name)}</b>`, `T${v.tier}`, W.statusPill(v.status), `<code>${esc(v.version)}</code>`,
    `${v.uptime30d}%`, `${v.latencyMs} ms`, esc(teamName(s, v.ownerTeamId)),
  ]));
  const prsTable = table(['PR', 'Repo', 'Title', 'Author', 'CI', 'Approvals', 'Status'], openPRs, (p) => W.tr([
    `<a href="/prs/${esc(p.id)}"><code>${esc(p.id)}</code> #${p.number}</a>`, `<code>${esc(repoName(s, p.repoId))}</code>`,
    esc(p.title), esc(userName(s, p.authorId)), W.statusPill(latestCI(s, p).status), `${p.approvals.length}`, W.statusPill(p.status),
  ]));
  const incTable = table(['Incident', 'Severity', 'Title', 'Service', 'Status'], openInc, (i) => W.tr([
    `<code>${esc(i.id)}</code>`, W.statusPill(i.severity), esc(i.title), `<code>${esc(svcName(s, i.serviceId))}</code>`, W.statusPill(i.status),
  ]));
  return W.page(ctx.app.config, {
    title: 'Dashboard', user: ctx.user, query: ctx.query,
    body: `
<h1>ForgeOps — engineering operations</h1>
${ctx.user ? '' : '<div class="flash flash-warn" role="alert">Public view. <a href="/login">Sign in</a> as a seeded persona (engineer, code owner, release manager, SRE, intern) to operate.</div>'}
<div class="grid grid3">
${W.kpi('Services', s.services.length, s.services.filter((v) => v.status !== 'healthy').length + ' unhealthy')}
${W.kpi('Open PRs', openPRs.length, 'awaiting review/merge')}
${W.kpi('Open incidents', openInc.length, 'on-call')}
${W.kpi('Repos', s.repos.length, 'git forge')}
</div>
${W.card('Service dashboard', servicesTable)}
${W.card('Open pull requests', prsTable)}
${W.card('Open incidents', incTable)}
<p class="muted">Terminal desktop console: <code>node deploy-console.js</code> in this directory. Docs: <a href="/failures">failure switches</a> · <code>fixtures/software/FAILURES.md</code>.</p>`,
  });
}
function latestCI(s, pr) {
  const runId = pr.ciRuns[pr.ciRuns.length - 1];
  return (s.ciRuns.find((r) => r.id === runId) || { status: 'none' });
}
function teamName(s, id) {
  const t = (s.teams || []).find((x) => x.id === id);
  return t ? t.name : id;
}
function svcName(s, id) {
  const v = (s.services || []).find((x) => x.id === id);
  return v ? v.name : id;
}

function repos(ctx) {
  const s = ctx.state;
  const t = table(['Repo', 'Language', 'Owner team', 'Reviewers', 'Staging', 'Production', 'Open PRs'], s.repos, (r) => W.tr([
    `<a href="/repos/${esc(r.id)}"><b>${esc(r.name)}</b></a>`, esc(r.language), esc(teamName(s, r.ownerTeamId)),
    r.reviewerIds.map((u) => esc(userName(s, u))).join(', '),
    `<code>${esc(r.currentStagingVersion)}</code>`, `<code>${esc(r.productionVersion)}</code>`,
    String(s.pullRequests.filter((p) => p.repoId === r.id && p.status === 'open').length),
  ]));
  return W.page(ctx.app.config, {
    title: 'Repositories', user: ctx.user, query: ctx.query,
    body: `<h1>Git forge — repositories</h1>${W.card('All repositories', t)}`,
  });
}

function repoPage(ctx) {
  const s = ctx.state;
  const repo = s.repos.find((r) => r.id === ctx.params.id);
  if (!repo) throw new R.AppError(404, 'repo_not_found', `Repository "${ctx.params.id}" does not exist.`);
  const prs = s.pullRequests.filter((p) => p.repoId === repo.id);
  const deps = s.deployments.filter((d) => d.repoId === repo.id).slice().reverse();
  const arts = s.artifacts.filter((a) => a.repoId === repo.id);
  const createForm = can(ctx, 'pr:create') ? W.card('Open a pull request', W.frm('/ui/prs/create', [
    W.hidden('repoId', repo.id),
    W.fld('Title', W.txt('title', '', { required: true, placeholder: 'Refund idempotency keys' })),
    W.fld('Source branch', W.txt('branch', (ctx.user ? ctx.user.username.split('.')[0] : 'user') + '/feature', { required: true })),
    W.fld('Base', W.txt('base', repo.defaultBranch)),
    W.fld('Additions / deletions', W.txt('additions', '120') + W.txt('deletions', '18')),
    W.fld('Summary', W.ta('summary', 'What does this change do?')),
  ], { submit: 'Open PR', note: 'Opening a PR automatically runs CI (lint, unit, integration) and requests review.' })) : '';
  const prsTable = table(['PR', 'Title', 'Author', 'CI', 'Approvals', 'Status'], prs, (p) => W.tr([
    `<a href="/prs/${esc(p.id)}"><code>${esc(p.id)}</code> #${p.number}</a>`, esc(p.title),
    esc(userName(s, p.authorId)), W.statusPill(latestCI(s, p).status), `${p.approvals.length}`, W.statusPill(p.status),
  ]));
  const depsTable = table(['Deployment', 'PR', 'Env', 'Version', 'Artifact', 'Status', 'Action'], deps, (d) => W.tr([
    `<code>${esc(d.id)}</code>`, d.prId ? `<code>${esc(d.prId)}</code>` : '—', W.statusPill(d.env === 'production' ? 'info' : d.status === 'succeeded' ? 'ok' : 'neutral') + ` ${d.env}`,
    `<code>${esc(d.version)}</code>`, `<code>${esc(d.artifactId)}</code>`, W.statusPill(d.status),
    promoteForm(ctx, d) || rollbackForm(ctx, d) || '—',
  ]));
  const artsTable = arts.length ? table(['Artifact', 'Digest', 'Size', 'Built from'], arts, (a) => W.tr([
    `<code>${esc(a.id)}</code>`, `<code>${esc(a.digest)}</code>`, `${(a.sizeKb / 1024).toFixed(1)} MB`, `<code>${esc(a.builtFromPr)}</code>`,
  ])) : '<p class="muted">No artifacts yet.</p>';
  return W.page(ctx.app.config, {
    title: repo.name, user: ctx.user, query: ctx.query,
    body: `
<h1><code>${esc(repo.name)}</code> <span class="muted">${esc(repo.language)} · base ${esc(repo.defaultBranch)} · owner ${esc(teamName(s, repo.ownerTeamId))}</span></h1>
${W.card('Versions', W.dl([
  ['Staging', `<code>${esc(repo.currentStagingVersion)}</code>`],
  ['Production', `<code>${esc(repo.productionVersion)}</code>`],
  ['Service', `<code>${esc(repo.serviceName)}</code>`],
]))}
${createForm}
${W.card('Pull requests', prsTable)}
${W.card('Deployments', depsTable)}
${W.card('Build artifacts (registry)', artsTable)}`,
  });
}
function promoteForm(ctx, d) {
  if (!can(ctx, 'deploy:promote') || d.env !== 'staging' || d.status !== 'succeeded') return '';
  return W.frm('/ui/deployments/promote', [W.hidden('deploymentId', d.id)], { submit: 'Promote to production', sec: false });
}
function rollbackForm(ctx, d) {
  if (!can(ctx, 'deploy:rollback') || d.env !== 'production' || d.status !== 'succeeded') return '';
  return W.frm('/ui/deployments/rollback', [W.hidden('deploymentId', d.id)], { submit: 'Rollback' });
}

function prPage(ctx) {
  const s = ctx.state;
  const pr = s.pullRequests.find((p) => p.id === ctx.params.id);
  if (!pr) throw new R.AppError(404, 'pr_not_found', `Pull request "${ctx.params.id}" does not exist.`);
  const repo = s.repos.find((r) => r.id === pr.repoId);
  const runs = pr.ciRuns.map((rid) => s.ciRuns.find((r) => r.id === rid)).filter(Boolean).reverse();
  const canApprove = can(ctx, 'pr:review') && pr.status === 'open';
  const canMerge = can(ctx, 'pr:merge') && pr.status === 'open';
  const canRerun = can(ctx, 'ci:run') && pr.status === 'open';
  const approveForm = canApprove ? W.card('Approve (code review)', W.frm('/ui/prs/approve', [
    W.hidden('prId', pr.id),
    W.fld('Review note', W.txt('note', 'LGTM')),
  ], { submit: 'Approve', note: 'Self-approval is blocked by policy — the author cannot approve their own PR.' })) : '';
  const mergeForm = canMerge ? W.card('Merge (release manager / code owner)', W.frm('/ui/prs/merge', [
    W.hidden('prId', pr.id),
  ], { submit: 'Merge & deploy to staging', note: 'Requires ≥1 approval and a passing CI run. Merging deploys the version to staging and creates a build artifact.' })) : '';
  const rerunForm = canRerun ? W.card('Re-run CI', W.frm('/ui/prs/ci', [W.hidden('prId', pr.id)], { submit: 'Re-run CI', sec: true })) : '';
  const approvalsHtml = pr.approvals.length
    ? `<ul>${pr.approvals.map((a) => `<li><b>${esc(userName(s, a.byId))}</b> approved <span class="muted">${esc(a.at)} — ${esc(a.note || '')}</span></li>`).join('')}</ul>`
    : '<p class="muted">No approvals yet.</p>';
  const runsHtml = runs.length ? runs.map((r) => W.card(`CI run <code>${esc(r.id)}</code> — ${W.statusPill(r.status)}`,
    table(['Step', 'Status', 'Duration'], r.steps, (st) => W.tr([esc(st.name), W.statusPill(st.status), `${st.durationSec}s`])))).join('')
    : '<p class="muted">No CI runs.</p>';
  return W.page(ctx.app.config, {
    title: `PR #${pr.number}`, user: ctx.user, query: ctx.query,
    body: `
<h1>PR #${pr.number} — ${esc(pr.title)} ${W.statusPill(pr.status)}</h1>
${W.card('Overview', W.dl([
  ['Repo', `<a href="/repos/${esc(repo.id)}"><code>${esc(repo.name)}</code></a>`],
  ['Author', esc(userName(s, pr.authorId))],
  ['Branch', `<code>${esc(pr.branch)}</code> → <code>${esc(pr.base)}</code>`],
  ['Diff', `+${pr.additions} / −${pr.deletions}`],
  ['Merged by', pr.mergedById ? esc(userName(s, pr.mergedById)) : '—'],
  ['Summary', esc(pr.summary || '')],
]))}
${W.card('Approvals', approvalsHtml)}
${approveForm}
${rerunForm}
${mergeForm}
${W.card('CI pipeline', runsHtml)}`,
  });
}

module.exports = { dashboard, repos, repo: repoPage, pr: prPage, userName, repoName, teamName, svcName, table, can, latestCI };
