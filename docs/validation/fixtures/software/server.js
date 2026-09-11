'use strict';
/**
 * ForgeOps — software/Google-like family entry point (VWO-002 synthetic fixture).
 * Run:  node server.js [--port 4102]     (bun server.js works identically)
 * Terminal desktop console: node deploy-console.js [--port 4102]
 */

const R = require('../_lib/runtime');
const seed = require('./seed');
const ops = require('./ops');
const infra = require('./ops-infra');
const ui = require('./ui');
const ui2 = require('./ui2');

const portArg = process.argv.includes('--port') ? process.argv[process.argv.indexOf('--port') + 1] : null;
const port = parseInt(portArg || process.env.FIXTURE_PORT || '4102', 10);

const failures = [
  { switch: 'service_unavailable', applies_to: 'any state-changing request', behavior: 'The synthetic "ci-runner-pool / deploy pipeline" dependency is treated as down.', symptom: 'HTTP 503 {error:"service_unavailable", service:"ci-runner-pool"}; reads still work.' },
  { switch: 'notification_failure', applies_to: 'PR open/approve/merge, promote, rollback, incident open/resolve', behavior: 'Operation succeeds; the in-app notification is stored as "failed".', symptom: 'HTTP 200 + warning; failed row in /notifications; notification.failed event.' },
  { switch: 'missing_asset', applies_to: 'POST /api/deployments/promote (+form twin), GET /api/artifacts/:id', behavior: 'The release artifact is treated as missing from the registry.', symptom: 'HTTP 404 {error:"missing_asset", artifactId}. Natural case: promoting a deployment whose artifact id is unknown.' },
  { switch: 'permission_denied', applies_to: 'any endpoint declaring a permission', behavior: 'Permission check fails even for authorized users.', symptom: 'HTTP 403 {error:"permission_denied", simulated:true}. Natural case: an engineer approving their own PR (self-approval policy), an intern promoting deployments.' },
  { switch: 'data_conflict', applies_to: 'merge, approve, re-run CI, promote, rollback, incident resolve, doc update', behavior: 'The record is treated as concurrently changed by someone else.', symptom: 'HTTP 409 {error:"data_conflict"}. Natural case: merging an already-merged PR; editing a doc on a stale expectedVersion.' },
  { switch: 'duplicate_event', applies_to: 'any state-changing request', behavior: 'Request signature recorded and rejected as a replay; explicit idempotencyKey replays rejected.', symptom: 'HTTP 409 {error:"duplicate_event"}. Natural case: same reviewer approving twice; opening a second PR from the same branch.' },
  { switch: 'stale_entitlement', applies_to: 'POST /api/deployments/promote (+form twin)', behavior: 'The production release entitlement is treated as expired; promotion blocked.', symptom: 'HTTP 409 {error:"stale_entitlement", entitlement:"release-license"} (simulated-only in this family).' },
];

const routes = [
  { kind: 'page', method: 'GET', pattern: '/', auth: false, handler: ui.dashboard },
  { kind: 'page', method: 'GET', pattern: '/repos', auth: false, handler: ui.repos },
  { kind: 'page', method: 'GET', pattern: '/repos/:id', auth: false, handler: ui.repo },
  { kind: 'page', method: 'GET', pattern: '/prs/:id', auth: false, handler: ui.pr },
  { kind: 'page', method: 'GET', pattern: '/deployments', auth: false, handler: ui2.deployments },
  { kind: 'page', method: 'GET', pattern: '/incidents', auth: false, handler: ui2.incidents },
  { kind: 'page', method: 'GET', pattern: '/services', auth: false, handler: ui2.services },
  { kind: 'page', method: 'GET', pattern: '/issues', auth: false, handler: ui2.issues },
  { kind: 'page', method: 'GET', pattern: '/docs', auth: false, handler: ui2.docs },
  { kind: 'page', method: 'GET', pattern: '/docs/:id', auth: false, handler: ui2.doc },
  { kind: 'page', method: 'GET', pattern: '/teams', auth: false, handler: ui2.teams },
  { kind: 'page', method: 'GET', pattern: '/notifications', handler: ui2.notifications },

  // read APIs
  { kind: 'api', method: 'GET', pattern: '/api/repos', auth: false, op: (ctx) => ({ repos: ctx.state.repos }) },
  { kind: 'api', method: 'GET', pattern: '/api/repos/:id', auth: false, op: (ctx) => {
    const repo = ctx.state.repos.find((r) => r.id === ctx.params.id);
    if (!repo) throw new R.AppError(404, 'repo_not_found', `Repository "${ctx.params.id}" does not exist.`);
    return {
      repo,
      pullRequests: ctx.state.pullRequests.filter((p) => p.repoId === repo.id),
      deployments: ctx.state.deployments.filter((d) => d.repoId === repo.id),
      artifacts: ctx.state.artifacts.filter((a) => a.repoId === repo.id),
    };
  } },
  { kind: 'api', method: 'GET', pattern: '/api/prs/:id', auth: false, op: (ctx) => {
    const pr = ctx.state.pullRequests.find((p) => p.id === ctx.params.id);
    if (!pr) throw new R.AppError(404, 'pr_not_found', `Pull request "${ctx.params.id}" does not exist.`);
    return { pullRequest: pr, ciRuns: pr.ciRuns.map((rid) => ctx.state.ciRuns.find((r) => r.id === rid)).filter(Boolean) };
  } },
  { kind: 'api', method: 'GET', pattern: '/api/deployments', auth: false, op: (ctx) => ({ deployments: ctx.state.deployments.slice().reverse() }) },
  { kind: 'api', method: 'GET', pattern: '/api/incidents', auth: false, op: (ctx) => ({ incidents: ctx.state.incidents }) },
  { kind: 'api', method: 'GET', pattern: '/api/services', auth: false, op: (ctx) => ({ services: ctx.state.services }) },
  { kind: 'api', method: 'GET', pattern: '/api/issues', auth: false, op: (ctx) => ({ issues: ctx.state.issues }) },
  { kind: 'api', method: 'GET', pattern: '/api/docs', auth: false, op: (ctx) => ({ docs: ctx.state.docs }) },
  { kind: 'api', method: 'GET', pattern: '/api/teams', auth: false, op: (ctx) => ({ teams: ctx.state.teams }) },
  { kind: 'api', method: 'GET', pattern: '/api/notifications', op: (ctx) => ({
    notifications: ctx.state.notifications.filter((n) => n.userId === ctx.user.id).slice().reverse(),
  }) },
  { kind: 'api', method: 'GET', pattern: '/api/artifacts/:id', auth: false, op: (ctx) => {
    if (ctx.failure === 'missing_asset') {
      throw new R.AppError(404, 'missing_asset', `Artifact "${ctx.params.id}" could not be resolved in the registry (simulated).`, { artifactId: ctx.params.id, simulated: true });
    }
    const a = ctx.state.artifacts.find((x) => x.id === ctx.params.id);
    if (!a) throw new R.AppError(404, 'missing_asset', `Artifact "${ctx.params.id}" does not exist.`, { artifactId: ctx.params.id });
    return { artifact: a };
  } },

  // mutations — API twins + HTML forms share ops
  { kind: 'api', method: 'POST', pattern: '/api/prs/create', perm: 'pr:create',
    naturalKey: (ctx) => `${ctx.body.repoId}:${ctx.body.branch}`, op: ops.createPullRequest },
  { kind: 'form', method: 'POST', pattern: '/ui/prs/create', perm: 'pr:create',
    naturalKey: (ctx) => `${ctx.body.repoId}:${ctx.body.branch}`, op: ops.createPullRequest, back: '/repos' },
  { kind: 'api', method: 'POST', pattern: '/api/prs/ci', perm: 'ci:run', op: ops.rerunCI },
  { kind: 'form', method: 'POST', pattern: '/ui/prs/ci', perm: 'ci:run', op: ops.rerunCI, back: '/prs' },
  { kind: 'api', method: 'POST', pattern: '/api/prs/approve', perm: 'pr:review', op: ops.approvePR },
  { kind: 'form', method: 'POST', pattern: '/ui/prs/approve', perm: 'pr:review', op: ops.approvePR, back: '/prs' },
  { kind: 'api', method: 'POST', pattern: '/api/prs/merge', perm: 'pr:merge', op: ops.mergePR },
  { kind: 'form', method: 'POST', pattern: '/ui/prs/merge', perm: 'pr:merge', op: ops.mergePR, back: '/prs' },
  { kind: 'api', method: 'POST', pattern: '/api/deployments/promote', perm: 'deploy:promote', op: infra.promoteDeployment },
  { kind: 'form', method: 'POST', pattern: '/ui/deployments/promote', perm: 'deploy:promote', op: infra.promoteDeployment, back: '/deployments' },
  { kind: 'api', method: 'POST', pattern: '/api/deployments/rollback', perm: 'deploy:rollback', op: infra.rollbackDeployment },
  { kind: 'form', method: 'POST', pattern: '/ui/deployments/rollback', perm: 'deploy:rollback', op: infra.rollbackDeployment, back: '/deployments' },
  { kind: 'api', method: 'POST', pattern: '/api/incidents/open', perm: 'incident:open', op: infra.openIncident },
  { kind: 'form', method: 'POST', pattern: '/ui/incidents/open', perm: 'incident:open', op: infra.openIncident, back: '/incidents' },
  { kind: 'api', method: 'POST', pattern: '/api/incidents/resolve', perm: 'incident:resolve', op: infra.resolveIncident },
  { kind: 'form', method: 'POST', pattern: '/ui/incidents/resolve', perm: 'incident:resolve', op: infra.resolveIncident, back: '/incidents' },
  { kind: 'api', method: 'POST', pattern: '/api/issues/create', perm: 'issue:create',
    naturalKey: (ctx) => `${ctx.body.repoId}:${ctx.body.title}`, op: ops.createIssue },
  { kind: 'form', method: 'POST', pattern: '/ui/issues/create', perm: 'issue:create',
    naturalKey: (ctx) => `${ctx.body.repoId}:${ctx.body.title}`, op: ops.createIssue, back: '/issues' },
  { kind: 'api', method: 'POST', pattern: '/api/issues/close', perm: 'issue:close', op: ops.closeIssue },
  { kind: 'form', method: 'POST', pattern: '/ui/issues/close', perm: 'issue:close', op: ops.closeIssue, back: '/issues' },
  { kind: 'api', method: 'POST', pattern: '/api/docs/update', perm: 'doc:write', op: infra.updateDoc },
  { kind: 'form', method: 'POST', pattern: '/ui/docs/update', perm: 'doc:write', op: infra.updateDoc, back: '/docs' },
];

const app = R.createApp({
  family: 'software',
  appName: 'forgeops',
  appTitle: 'ForgeOps',
  tagline: 'Git forge · CI · deployments · incidents — synthetic validation fixture',
  port,
  rootDir: __dirname,
  seedFn: seed.build,
  serviceName: 'ci-runner-pool',
  accent: '#0f766e', accentSoft: '#e6f2f0',
  nav: [['/', 'Dashboard'], ['/repos', 'Repos'], ['/deployments', 'Deployments'], ['/incidents', 'Incidents'], ['/services', 'Services'], ['/issues', 'Issues'], ['/docs', 'Docs'], ['/teams', 'Teams'], ['/notifications', 'Notifications'], ['/events', 'Events'], ['/failures', 'Failure switches']],
  failures,
  routes,
});

app.start();
