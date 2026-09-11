'use strict';
/**
 * RidePilot — ride-share family entry point (VWO-002 synthetic fixture).
 * Run:  node server.js [--port 4103]     (bun server.js works identically)
 */

const R = require('../_lib/runtime');
const seed = require('./seed');
const ops = require('./ops');
const ops2 = require('./ops-ops');
const ui = require('./ui');

const portArg = process.argv.includes('--port') ? process.argv[process.argv.indexOf('--port') + 1] : null;
const port = parseInt(portArg || process.env.FIXTURE_PORT || '4103', 10);

const failures = [
  { switch: 'service_unavailable', applies_to: 'any state-changing request', behavior: 'The synthetic "payout-gateway / dispatch" dependency is treated as down.', symptom: 'HTTP 503 {error:"service_unavailable", service:"dispatch-payout-gateway"}; reads still work.' },
  { switch: 'notification_failure', applies_to: 'intake, screening, escalation, surge broadcast, incident report/resolve', behavior: 'Operation succeeds; the notification/broadcast record is stored "failed".', symptom: 'HTTP 200 + warning; failed row in /notifications or broadcasts; notification.failed event.' },
  { switch: 'missing_asset', applies_to: 'screening approval (and naturally: applications with a missing document)', behavior: 'A required onboarding document (license/insurance) is treated as not on file.', symptom: 'HTTP 404 {error:"missing_asset", document:"insurance"}; natural case: da-0302 (Elena Petrova) is missing insurance.' },
  { switch: 'permission_denied', applies_to: 'any endpoint declaring a permission', behavior: 'Permission check fails even for authorized users.', symptom: 'HTTP 403 {error:"permission_denied", simulated:true}; natural case: a support agent screening applications, finance adjusting surge.' },
  { switch: 'data_conflict', applies_to: 'screening, escalation, resolve, surge, payout approval, incident resolve', behavior: 'Record treated as concurrently decided by someone else.', symptom: 'HTTP 409 {error:"data_conflict"}; natural case: screening an already-rejected application, approving a processed payout.' },
  { switch: 'duplicate_event', applies_to: 'any state-changing request', behavior: 'Request signature recorded + rejected as replay; idempotencyKey replays rejected.', symptom: 'HTTP 409 {error:"duplicate_event"}; natural case: public intake submitting the same vehicle plate twice.' },
  { switch: 'stale_entitlement', applies_to: 'driver activation, payout approval', behavior: 'Regional driver permit/entitlement treated as expired.', symptom: 'HTTP 409 {error:"stale_entitlement"}; natural case: activating da-0303 (permit expired 2026-08-01), approving pay-0903 (Lena Marsh, permit ended 2026-07-01).' },
];

const routes = [
  { kind: 'page', method: 'GET', pattern: '/', auth: false, handler: ui.dashboard },
  { kind: 'page', method: 'GET', pattern: '/onboarding', auth: false, handler: ui.onboarding },
  { kind: 'page', method: 'GET', pattern: '/intake', auth: false, handler: ui.intake },
  { kind: 'page', method: 'GET', pattern: '/support', auth: false, handler: ui.support },
  { kind: 'page', method: 'GET', pattern: '/ops', auth: false, handler: ui.ops },
  { kind: 'page', method: 'GET', pattern: '/map', auth: false, handler: ui.map },
  { kind: 'page', method: 'GET', pattern: '/payments', auth: false, handler: ui.payments },
  { kind: 'page', method: 'GET', pattern: '/incidents', auth: false, handler: ui.incidents },
  { kind: 'page', method: 'GET', pattern: '/notifications', handler: ui.notifications },

  // read APIs
  { kind: 'api', method: 'GET', pattern: '/api/regions', auth: false, op: (ctx) => ({ regions: ctx.state.regions }) },
  { kind: 'api', method: 'GET', pattern: '/api/drivers', auth: false, op: (ctx) => ({ drivers: ctx.state.drivers }) },
  { kind: 'api', method: 'GET', pattern: '/api/applications', auth: false, op: (ctx) => ({ applications: ctx.state.driverApplications }) },
  { kind: 'api', method: 'GET', pattern: '/api/applications/:id', auth: false, op: (ctx) => {
    const a = ctx.state.driverApplications.find((x) => x.id === ctx.params.id);
    if (!a) throw new R.AppError(404, 'application_not_found', `Application "${ctx.params.id}" does not exist.`);
    return { application: a };
  } },
  { kind: 'api', method: 'GET', pattern: '/api/tickets', auth: false, op: (ctx) => ({ tickets: ctx.state.supportTickets }) },
  { kind: 'api', method: 'GET', pattern: '/api/trips', auth: false, op: (ctx) => ({ trips: ctx.state.trips }) },
  { kind: 'api', method: 'GET', pattern: '/api/payments', auth: false, op: (ctx) => ({ payments: ctx.state.payments }) },
  { kind: 'api', method: 'GET', pattern: '/api/incidents', auth: false, op: (ctx) => ({ incidents: ctx.state.incidents }) },
  { kind: 'api', method: 'GET', pattern: '/api/broadcasts', auth: false, op: (ctx) => ({ broadcasts: ctx.state.broadcasts }) },
  { kind: 'api', method: 'GET', pattern: '/api/notifications', op: (ctx) => ({
    notifications: ctx.state.notifications.filter((n) => n.userId === ctx.user.id).slice().reverse(),
  }) },

  // mutations — API twins + HTML forms share ops
  { kind: 'api', method: 'POST', pattern: '/api/intake/apply', auth: false,
    naturalKey: (ctx) => `plate:${String(ctx.body.plate || '').toUpperCase()}`, op: ops.submitDriverApplication },
  { kind: 'form', method: 'POST', pattern: '/ui/intake/apply', auth: false,
    naturalKey: (ctx) => `plate:${String(ctx.body.plate || '').toUpperCase()}`, op: ops.submitDriverApplication, back: '/intake' },
  { kind: 'api', method: 'POST', pattern: '/api/applications/screen', perm: 'driver:screen', op: ops.screenDriverApplication },
  { kind: 'form', method: 'POST', pattern: '/ui/applications/screen', perm: 'driver:screen', op: ops.screenDriverApplication, back: '/onboarding' },
  { kind: 'api', method: 'POST', pattern: '/api/drivers/activate', perm: 'driver:activate', op: ops.activateDriver },
  { kind: 'form', method: 'POST', pattern: '/ui/drivers/activate', perm: 'driver:activate', op: ops.activateDriver, back: '/onboarding' },
  { kind: 'api', method: 'POST', pattern: '/api/tickets/respond', perm: 'ticket:respond', op: ops.respondTicket },
  { kind: 'form', method: 'POST', pattern: '/ui/tickets/respond', perm: 'ticket:respond', op: ops.respondTicket, back: '/support' },
  { kind: 'api', method: 'POST', pattern: '/api/tickets/escalate', perm: 'ticket:escalate', op: ops.escalateTicket },
  { kind: 'form', method: 'POST', pattern: '/ui/tickets/escalate', perm: 'ticket:escalate', op: ops.escalateTicket, back: '/support' },
  { kind: 'api', method: 'POST', pattern: '/api/tickets/resolve', perm: 'ticket:resolve', op: ops.resolveTicket },
  { kind: 'form', method: 'POST', pattern: '/ui/tickets/resolve', perm: 'ticket:resolve', op: ops.resolveTicket, back: '/support' },
  { kind: 'api', method: 'POST', pattern: '/api/regions/surge', perm: 'surge:adjust', op: ops2.adjustSurge },
  { kind: 'form', method: 'POST', pattern: '/ui/regions/surge', perm: 'surge:adjust', op: ops2.adjustSurge, back: '/ops' },
  { kind: 'api', method: 'POST', pattern: '/api/payments/approve', perm: 'payout:approve', op: ops2.approvePayout },
  { kind: 'form', method: 'POST', pattern: '/ui/payments/approve', perm: 'payout:approve', op: ops2.approvePayout, back: '/payments' },
  { kind: 'api', method: 'POST', pattern: '/api/incidents/report', perm: 'incident:report', op: ops2.reportIncident },
  { kind: 'form', method: 'POST', pattern: '/ui/incidents/report', perm: 'incident:report', op: ops2.reportIncident, back: '/incidents' },
  { kind: 'api', method: 'POST', pattern: '/api/incidents/resolve', perm: 'incident:resolve', op: ops2.resolveIncident },
  { kind: 'form', method: 'POST', pattern: '/ui/incidents/resolve', perm: 'incident:resolve', op: ops2.resolveIncident, back: '/incidents' },
];

const app = R.createApp({
  family: 'rideshare',
  appName: 'ridepilot',
  appTitle: 'RidePilot',
  tagline: 'Driver onboarding · support · ops · trips · payments — synthetic validation fixture',
  port,
  rootDir: __dirname,
  seedFn: seed.build,
  serviceName: 'dispatch-payout-gateway',
  accent: '#be185d', accentSoft: '#fce7f1',
  nav: [['/', 'Dashboard'], ['/onboarding', 'Onboarding'], ['/intake', 'Apply to drive'], ['/support', 'Support'], ['/ops', 'Ops'], ['/map', 'Map'], ['/payments', 'Payments'], ['/incidents', 'Incidents'], ['/notifications', 'Notifications'], ['/events', 'Events'], ['/failures', 'Failure switches']],
  failures,
  routes,
});

app.start();
