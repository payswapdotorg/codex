'use strict';
/**
 * FlowMart — marketplace family entry point (VWO-002 synthetic fixture).
 * Run:  node server.js [--port 4105]     (bun server.js works identically)
 *
 * SCOPE (VWO-002 forbidden list): FlowMart is a store surface — catalog,
 * listings, versions, provenance, installs, entitlements, upgrades, rollback.
 * It contains NO workflow semantics and NO execution engine: packages are
 * opaque artifacts (manifest + sha256 digest + provenance).
 */

const R = require('../_lib/runtime');
const seed = require('./seed');
const ops = require('./ops');
const inst = require('./ops-install');
const ui = require('./ui');

const portArg = process.argv.includes('--port') ? process.argv[process.argv.indexOf('--port') + 1] : null;
const port = parseInt(portArg || process.env.FIXTURE_PORT || '4105', 10);

const failures = [
  { switch: 'service_unavailable', applies_to: 'any state-changing request', behavior: 'The synthetic "registry blob store / payment hook" dependency is treated as down.', symptom: 'HTTP 503 {error:"service_unavailable", service:"registry-blob-store"}; reads (catalog/search) still work.' },
  { switch: 'notification_failure', applies_to: 'listing publish, version publish, install, upgrade, rollback, entitlement grant/revoke, review', behavior: 'Operation succeeds; the in-app notification is stored "failed".', symptom: 'HTTP 200 + warning; failed row in /notifications; notification.failed event.' },
  { switch: 'missing_asset', applies_to: 'package install', behavior: 'The package archive is treated as missing from the registry blob store.', symptom: 'HTTP 404 {error:"missing_asset", version}; natural case: installing an unknown version.' },
  { switch: 'permission_denied', applies_to: 'any endpoint declaring a permission', behavior: 'Permission check fails even for authorized users.', symptom: 'HTTP 403 {error:"permission_denied", simulated:true}; natural cases: an org member (not admin) installing, a buyer publishing listings, a publisher granting entitlements.' },
  { switch: 'data_conflict', applies_to: 'version publish, install (expectedVersion), upgrade (expectedCurrentVersion), rollback, entitlement revoke, provenance verify', behavior: 'Record treated as concurrently changed / optimistic-concurrency mismatch / digest mismatch.', symptom: 'HTTP 409 {error:"data_conflict", currentVersion?}; natural cases: installing an out-of-date expectedVersion, re-revoking an entitlement.' },
  { switch: 'duplicate_event', applies_to: 'any state-changing request', behavior: 'Request signature recorded + rejected as replay; idempotencyKey replays rejected.', symptom: 'HTTP 409 {error:"duplicate_event"}; natural cases: duplicate listing slug, duplicate version, second install for the same org, second review from the same org.' },
  { switch: 'stale_entitlement', applies_to: 'install, upgrade, rollback, configure', behavior: 'Entitlement treated as expired/revoked; commercial operations blocked.', symptom: 'HTTP 409 {error:"stale_entitlement", entitlementId, validUntil}; natural cases: Northwind installing pk-101 (trial expired 2026-08-01), any operation after an entitlement is revoked.' },
];

const routes = [
  { kind: 'page', method: 'GET', pattern: '/', auth: false, handler: ui.dashboard },
  { kind: 'page', method: 'GET', pattern: '/catalog', auth: false, handler: ui.catalog },
  { kind: 'page', method: 'GET', pattern: '/listings/:slug', auth: false, handler: ui.listing },
  { kind: 'page', method: 'GET', pattern: '/publisher', auth: false, handler: ui.publisher },
  { kind: 'page', method: 'GET', pattern: '/installs', auth: false, handler: ui.installs },
  { kind: 'page', method: 'GET', pattern: '/entitlements', auth: false, handler: ui.entitlements },
  { kind: 'page', method: 'GET', pattern: '/notifications', handler: ui.notifications },

  // read APIs
  { kind: 'api', method: 'GET', pattern: '/api/catalog', auth: false, op: (ctx) => ops.searchCatalog(ctx) },
  { kind: 'api', method: 'GET', pattern: '/api/packages/:id', auth: false, op: (ctx) => {
    const pkg = ctx.state.packages.find((p) => p.id === ctx.params.id || p.slug === ctx.params.id);
    if (!pkg) throw new R.AppError(404, 'package_not_found', `Package "${ctx.params.id}" does not exist.`);
    return { package: pkg, installs: ctx.state.installs.filter((i) => i.packageId === pkg.id), entitlements: ctx.state.entitlements.filter((e) => e.packageId === pkg.id) };
  } },
  { kind: 'api', method: 'GET', pattern: '/api/installs', op: (ctx) => {
    const orgId = ops.orgForUser(ctx.state, ctx.user.id);
    return { orgId, installs: ctx.state.installs.filter((i) => !orgId || i.orgId === orgId) };
  } },
  { kind: 'api', method: 'GET', pattern: '/api/entitlements', op: (ctx) => ({
    entitlements: ctx.state.entitlements.filter((e) => {
      const orgId = ops.orgForUser(ctx.state, ctx.user.id);
      return !orgId || e.orgId === orgId || ctx.user.permissions.includes('entitlement:grant');
    }),
  }) },
  { kind: 'api', method: 'GET', pattern: '/api/orgs', auth: false, op: (ctx) => ({ orgs: ctx.state.orgs }) },
  { kind: 'api', method: 'GET', pattern: '/api/reviews', auth: false, op: (ctx) => ({ reviews: ctx.state.reviews }) },
  { kind: 'api', method: 'GET', pattern: '/api/notifications', op: (ctx) => ({
    notifications: ctx.state.notifications.filter((n) => n.userId === ctx.user.id).slice().reverse(),
  }) },

  // mutations — API twins + HTML forms share ops
  { kind: 'api', method: 'POST', pattern: '/api/listings/publish', perm: 'listing:publish',
    naturalKey: (ctx) => `slug:${String(ctx.body.slug || ctx.body.name || '').toLowerCase()}`, op: ops.publishListing },
  { kind: 'form', method: 'POST', pattern: '/ui/listings/publish', perm: 'listing:publish',
    naturalKey: (ctx) => `slug:${String(ctx.body.slug || ctx.body.name || '').toLowerCase()}`, op: ops.publishListing, back: '/publisher' },
  { kind: 'api', method: 'POST', pattern: '/api/versions/publish', perm: 'version:publish', op: ops.publishVersion },
  { kind: 'form', method: 'POST', pattern: '/ui/versions/publish', perm: 'version:publish', op: ops.publishVersion, back: '/publisher' },
  { kind: 'api', method: 'POST', pattern: '/api/provenance/verify', perm: 'audit:verify', op: ops.verifyProvenance },
  { kind: 'form', method: 'POST', pattern: '/ui/provenance/verify', perm: 'audit:verify', op: ops.verifyProvenance, back: '/catalog' },
  { kind: 'api', method: 'POST', pattern: '/api/reviews/create', perm: 'review:write',
    naturalKey: (ctx) => `review:${ctx.user.id}:${ctx.body.packageId || ctx.body.slug}`, op: ops.writeReview },
  { kind: 'form', method: 'POST', pattern: '/ui/reviews/create', perm: 'review:write',
    naturalKey: (ctx) => `review:${ctx.user.id}:${ctx.body.packageId || ctx.body.slug}`, op: ops.writeReview, back: '/catalog' },
  { kind: 'api', method: 'POST', pattern: '/api/installs/install', perm: 'install:manage', op: inst.installPackage },
  { kind: 'form', method: 'POST', pattern: '/ui/installs/install', perm: 'install:manage', op: inst.installPackage, back: '/installs' },
  { kind: 'api', method: 'POST', pattern: '/api/installs/configure', perm: 'install:configure', op: inst.configureInstall },
  { kind: 'form', method: 'POST', pattern: '/ui/installs/configure', perm: 'install:configure', op: inst.configureInstall, back: '/installs' },
  { kind: 'api', method: 'POST', pattern: '/api/installs/upgrade', perm: 'install:manage', op: inst.upgradeInstall },
  { kind: 'form', method: 'POST', pattern: '/ui/installs/upgrade', perm: 'install:manage', op: inst.upgradeInstall, back: '/installs' },
  { kind: 'api', method: 'POST', pattern: '/api/installs/rollback', perm: 'install:manage', op: inst.rollbackInstall },
  { kind: 'form', method: 'POST', pattern: '/ui/installs/rollback', perm: 'install:manage', op: inst.rollbackInstall, back: '/installs' },
  { kind: 'api', method: 'POST', pattern: '/api/entitlements/revoke', perm: 'entitlement:revoke', op: inst.revokeEntitlement },
  { kind: 'form', method: 'POST', pattern: '/ui/entitlements/revoke', perm: 'entitlement:revoke', op: inst.revokeEntitlement, back: '/entitlements' },
  { kind: 'api', method: 'POST', pattern: '/api/entitlements/grant', perm: 'entitlement:grant', op: inst.grantEntitlement },
  { kind: 'form', method: 'POST', pattern: '/ui/entitlements/grant', perm: 'entitlement:grant', op: inst.grantEntitlement, back: '/entitlements' },
];

const app = R.createApp({
  family: 'marketplace',
  appName: 'flowmart',
  appTitle: 'FlowMart',
  tagline: 'Automation package marketplace — catalog · installs · entitlements — synthetic validation fixture',
  port,
  rootDir: __dirname,
  seedFn: seed.build,
  serviceName: 'registry-blob-store',
  accent: '#047857', accentSoft: '#e3f4ee',
  nav: [['/', 'Dashboard'], ['/catalog', 'Catalog'], ['/publisher', 'Publisher'], ['/installs', 'Installs'], ['/entitlements', 'Entitlements'], ['/notifications', 'Notifications'], ['/events', 'Events'], ['/failures', 'Failure switches']],
  failures,
  routes,
});

app.start();
