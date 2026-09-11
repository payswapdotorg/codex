'use strict';
/**
 * SiteBuild — construction family entry point (VWO-002 synthetic fixture).
 * Run:  node server.js [--port 4101]     (bun server.js works identically)
 * Docs: docs/validation/fixtures/construction/FAILURES.md + fixtures/README.md
 */

const R = require('../_lib/runtime');
const seed = require('./seed');
const ops = require('./ops');
const ui = require('./ui');

const portArg = process.argv.includes('--port') ? process.argv[process.argv.indexOf('--port') + 1] : null;
const port = parseInt(portArg || process.env.FIXTURE_PORT || '4101', 10);

const failures = [
  { switch: 'service_unavailable', applies_to: 'any state-changing request (API or form)',
    behavior: 'The synthetic "site-photo-store" dependency is treated as down; the mutation is rejected before touching state.',
    symptom: 'HTTP 503 {error:"service_unavailable", service:"site-photo-store", simulated:true}. Reads (GET) still work.' },
  { switch: 'notification_failure', applies_to: 'operations that notify a user (progress submit, PR decision, invoice approval, incident report/close)',
    behavior: 'The operation succeeds, but the in-app notification record is stored with status "failed" instead of "delivered".',
    symptom: 'HTTP 200 with a "warning" field; notification row in /notifications shows failed; event notification.failed in /events.' },
  { switch: 'missing_asset', applies_to: 'POST /api/progress, POST /ui/progress, GET /api/assets/:id',
    behavior: 'A referenced photo asset is treated as unresolvable (dangling reference) even if the id exists.',
    symptom: 'HTTP 404 {error:"missing_asset", assetId:...}. Natural case: submitting an unknown photo id fails the same way.' },
  { switch: 'permission_denied', applies_to: 'any endpoint that declares a permission (forms and APIs)',
    behavior: 'The permission check fails even for a user who would normally pass.',
    symptom: 'HTTP 403 {error:"permission_denied", required:"<perm>", simulated:true}. Natural case: a foreman calling a po:approve endpoint.' },
  { switch: 'data_conflict', applies_to: 'POST /api/procurement/decide, /api/invoices/approve, /api/safety/close (+ form twins)',
    behavior: 'The record is treated as already decisioned/processed by someone else.',
    symptom: 'HTTP 409 {error:"data_conflict", currentStatus:...}. Natural case: deciding the same purchase request twice.' },
  { switch: 'duplicate_event', applies_to: 'any state-changing request',
    behavior: 'The request signature is recorded and rejected as a replay; replays with an explicit idempotencyKey are always rejected as duplicates.',
    symptom: 'HTTP 409 {error:"duplicate_event", key:...}. Natural case: POSTing a second progress report for the same project+date.' },
  { switch: 'stale_entitlement', applies_to: 'POST /api/procurement/decide (approve) and the form twin',
    behavior: 'The vendor contract is treated as expired, blocking approval.',
    symptom: 'HTTP 409 {error:"stale_entitlement", vendorId, contractValidUntil}. Natural case: approving prq-5001 (SteelCo contract expired 2026-08-01 in the seed).' },
];

const routes = [
  { kind: 'page', method: 'GET', pattern: '/', auth: false, handler: ui.dashboard },
  { kind: 'page', method: 'GET', pattern: '/projects', auth: false, handler: ui.projects },
  { kind: 'page', method: 'GET', pattern: '/projects/:id', auth: false, handler: ui.project },
  { kind: 'page', method: 'GET', pattern: '/procurement', auth: false, handler: ui.procurement },
  { kind: 'page', method: 'GET', pattern: '/safety', auth: false, handler: ui.safety },
  { kind: 'page', method: 'GET', pattern: '/notifications', handler: ui.notifications },

  // read APIs
  { kind: 'api', method: 'GET', pattern: '/api/projects', auth: false, op: (ctx) => ({ projects: ctx.state.projects }) },
  { kind: 'api', method: 'GET', pattern: '/api/projects/:id', auth: false, op: (ctx) => {
    const d = ops.projectDetail(ctx.state, ctx.params.id);
    if (!d) throw new R.AppError(404, 'project_not_found', `Project "${ctx.params.id}" does not exist.`);
    return d;
  } },
  { kind: 'api', method: 'GET', pattern: '/api/notifications', op: (ctx) => ({
    notifications: ctx.state.notifications.filter((n) => n.userId === ctx.user.id).slice().reverse(),
  }) },
  { kind: 'api', method: 'GET', pattern: '/api/assets/:id', auth: false, op: (ctx) => {
    if (ctx.failure === 'missing_asset') {
      throw new R.AppError(404, 'missing_asset', `Asset "${ctx.params.id}" could not be resolved (simulated dangling reference).`, { assetId: ctx.params.id, simulated: true });
    }
    const a = ctx.state.assets.find((x) => x.id === ctx.params.id);
    if (!a) throw new R.AppError(404, 'missing_asset', `Asset "${ctx.params.id}" does not exist.`, { assetId: ctx.params.id });
    return { asset: a };
  } },

  // mutations: API twins + HTML forms share the same ops
  { kind: 'api', method: 'POST', pattern: '/api/progress', perm: 'progress:create',
    naturalKey: (ctx) => `${ctx.body.projectId}:${String(ctx.body.reportDate || '').slice(0, 10)}`, op: ops.submitProgress },
  { kind: 'form', method: 'POST', pattern: '/ui/progress', perm: 'progress:create',
    naturalKey: (ctx) => `${ctx.body.projectId}:${String(ctx.body.reportDate || '').slice(0, 10)}`, op: ops.submitProgress, back: '/projects' },
  { kind: 'api', method: 'POST', pattern: '/api/assets/upload', perm: 'progress:create', op: ops.uploadPhoto },
  { kind: 'form', method: 'POST', pattern: '/ui/assets/upload', perm: 'progress:create', op: ops.uploadPhoto, back: '/projects' },
  { kind: 'api', method: 'POST', pattern: '/api/procurement/decide', perm: 'po:approve', op: ops.decidePurchaseRequest },
  { kind: 'form', method: 'POST', pattern: '/ui/procurement/decide', perm: 'po:approve', op: ops.decidePurchaseRequest, back: '/procurement' },
  { kind: 'api', method: 'POST', pattern: '/api/invoices/approve', perm: 'invoice:approve', op: ops.approveInvoice },
  { kind: 'form', method: 'POST', pattern: '/ui/invoices/approve', perm: 'invoice:approve', op: ops.approveInvoice, back: '/procurement' },
  { kind: 'api', method: 'POST', pattern: '/api/safety/report', perm: 'safety:report', op: ops.reportSafetyIncident },
  { kind: 'form', method: 'POST', pattern: '/ui/safety/report', perm: 'safety:report', op: ops.reportSafetyIncident, back: '/safety' },
  { kind: 'api', method: 'POST', pattern: '/api/safety/close', perm: 'safety:close', op: ops.closeSafetyIncident },
  { kind: 'form', method: 'POST', pattern: '/ui/safety/close', perm: 'safety:close', op: ops.closeSafetyIncident, back: '/safety' },
];

const app = R.createApp({
  family: 'construction',
  appName: 'sitebuild',
  appTitle: 'SiteBuild Field Suite',
  tagline: 'Construction project operations — synthetic validation fixture',
  port,
  rootDir: __dirname,
  seedFn: seed.build,
  serviceName: 'site-photo-store',
  accent: '#b45309', accentSoft: '#fdf3e7',
  nav: [['/', 'Dashboard'], ['/projects', 'Projects'], ['/procurement', 'Procurement'], ['/safety', 'Safety'], ['/notifications', 'Notifications'], ['/events', 'Events'], ['/failures', 'Failure switches']],
  failures,
  routes,
});

app.start();
