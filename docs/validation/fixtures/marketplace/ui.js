'use strict';
/**
 * FlowMart server-rendered UI — dashboard, catalog/search, listing detail with
 * provenance panel, publisher console, org installs, entitlements admin.
 */

const W = require('../_lib/web');
const R = require('../_lib/runtime');
const C = require('./ops');
const I = require('./ops-install');
const esc = W.esc;

function userName(s, id) {
  const u = R.findUserById(s, id);
  return u ? u.name : (id || '—');
}
function table(headers, arr, rowFn) { return W.tbl(headers, arr.map(rowFn).join('')); }
function can(ctx, perm) { return ctx.user && ctx.user.permissions.includes(perm); }
function orgOf(ctx) {
  if (!ctx.user) return null;
  return C.orgForUser(ctx.state, ctx.user.id);
}

function dashboard(ctx) {
  const s = ctx.state;
  const orgId = orgOf(ctx);
  const myInstalls = orgId ? s.installs.filter((i) => i.orgId === orgId) : [];
  const myEnts = orgId ? s.entitlements.filter((e) => e.orgId === orgId) : [];
  const installsTable = myInstalls.length ? table(['Install', 'Package', 'Version', 'Status', 'Entitlement', 'Config'], myInstalls, (i) => W.tr([
    `<code>${esc(i.id)}</code>`, pkgLink(s, i.packageId), `<code>${esc(i.version)}</code>`, W.statusPill(i.status),
    entBadge(ctx, i.entitlementId), `<code>${esc(JSON.stringify(i.config))}</code>`,
  ])) : `<p class="muted">${orgId ? 'No installs yet — browse the <a href="/catalog">catalog</a>.' : 'Sign in as an org member to see your organization’s installs.'}</p>`;
  const entsTable = myEnts.length ? table(['Entitlement', 'Package', 'Type', 'Seats', 'Status', 'Valid until'], myEnts, (e) => W.tr([
    `<code>${esc(e.id)}</code>`, pkgLink(s, e.packageId), esc(e.type), e.seats || '—',
    W.statusPill(entStatus(ctx, e)), esc(e.validUntil),
  ])) : '';
  return W.page(ctx.app.config, {
    title: 'Dashboard', user: ctx.user, query: ctx.query,
    body: `
<h1>FlowMart — automation package marketplace</h1>
${ctx.user ? '' : '<div class="flash flash-warn" role="alert">Public view. <a href="/login">Sign in</a> as a seeded persona (publisher, admin, org admin, org member, auditor) to operate.</div>'}
<div class="grid grid3">
${W.kpi('Listings', s.packages.length, 'catalog')}
${W.kpi('Published versions', s.packages.reduce((a, p) => a + p.versions.length, 0), 'immutable')}
${W.kpi('Installs (all orgs)', s.installs.length, 'active')}
${W.kpi('Entitlements', s.entitlements.length, s.entitlements.filter((e) => entStatus(ctx, e) !== 'active').length + ' inactive')}
</div>
${W.card(orgId ? 'Your organization installs' : 'Installs', installsTable)}
${entsTable ? W.card('Your organization entitlements', entsTable) : ''}
<p class="muted"><a href="/catalog">Catalog &amp; search</a> · <a href="/publisher">Publisher console</a> · <a href="/entitlements">Entitlements</a> · <a href="/failures">failure switches</a> · repository: <code>docs/validation/fixtures/marketplace/</code>.</p>`,
  });
}
function pkgLink(s, id) {
  const p = (s.packages || []).find((x) => x.id === id);
  return p ? `<a href="/listings/${esc(p.slug)}">${esc(p.name)}</a>` : id;
}
function entStatus(ctx, e) {
  if (e.status !== 'active') return e.status;
  return C.isPast(ctx, e.validUntil) ? 'expired' : 'active';
}
function entBadge(ctx, id) {
  const e = (ctx.state.entitlements || []).find((x) => x.id === id);
  if (!e) return '—';
  return `<code>${esc(e.id)}</code> ${W.statusPill(entStatus(ctx, e))}`;
}

function catalog(ctx) {
  const s = ctx.state;
  const q = ctx.query.get('q') || '';
  const cat = ctx.query.get('category') || '';
  const canInstall = can(ctx, 'install:manage');
  const results = s.packages.filter((p) =>
    (!cat || p.category === cat) &&
    (!q || `${p.name} ${p.summary} ${p.tags.join(' ')}`.toLowerCase().includes(q.toLowerCase())));
  const rows = results.map((p) => W.tr([
    `<a href="/listings/${esc(p.slug)}"><b>${esc(p.name)}</b></a>`, `<code>${esc(p.slug)}</code>`, esc(p.category),
    esc(p.summary), esc(p.licenseModel), `<code>${esc(C.latestVersion(p).version)}</code>`,
    `${p.stats.installs} · ★${p.stats.stars} (${p.stats.reviews})`,
    canInstall ? installForm(ctx, p) : `<a class="btn sec mini" href="/listings/${esc(p.slug)}">View</a>`,
  ])).join('');
  return W.page(ctx.app.config, {
    title: 'Catalog', user: ctx.user, query: ctx.query,
    body: `<h1>Catalog</h1>
${W.card('Search the catalog', `<form method="get" action="/catalog" class="frm" style="grid-template-columns:2fr 1fr auto;max-width:720px">
<input name="q" value="${esc(q)}" placeholder="Search packages (e.g. invoice, triage)">
<select name="category"><option value="">All categories</option>${s.packages.map((p) => p.category).filter((v, i, a) => a.indexOf(v) === i).map((c) => `<option value="${esc(c)}"${c === cat ? ' selected' : ''}>${esc(c)}</option>`).join('')}</select>
<button class="btn" type="submit">Search</button></form>`)}
${W.card(`Results (${results.length})`, W.tbl(['Package', 'Slug', 'Category', 'Summary', 'License', 'Latest', 'Installs · stars', 'Action'], rows))}`,
  });
}
function installForm(ctx, p) {
  const latest = C.latestVersion(p);
  return W.frm('/ui/installs/install', [
    W.hidden('packageId', p.id),
    W.hidden('expectedVersion', latest.version),
  ], { submit: 'Install ' + latest.version, note: p.licenseModel === 'free' ? 'Free package.' : 'First install auto-grants a 30-day trial entitlement.' });
}

function listing(ctx) {
  const s = ctx.state;
  const pkg = s.packages.find((p) => p.slug === ctx.params.slug);
  if (!pkg) throw new R.AppError(404, 'listing_not_found', `Listing "${ctx.params.slug}" does not exist.`);
  const latest = C.latestVersion(pkg);
  const canInstall = can(ctx, 'install:manage');
  const canPublishVersion = can(ctx, 'version:publish');
  const canVerify = can(ctx, 'audit:verify');
  const canReview = can(ctx, 'review:write');
  const reviews = s.reviews.filter((r) => r.packageId === pkg.id);
  const overview = W.dl([
    ['ID', `<code>${esc(pkg.id)}</code>`], ['Publisher', esc(userName(s, pkg.publisherId))],
    ['Category', esc(pkg.category)], ['License model', esc(pkg.licenseModel)],
    ['Latest version', `<code>${esc(latest.version)}</code>`], ['Latest digest', `<code>${esc(latest.digest)}</code>`],
    ['Stats', `${pkg.stats.installs} installs · ★${pkg.stats.stars} · ${pkg.stats.reviews} reviews`],
    ['Tags', pkg.tags.map((t) => W.pill('info', t)).join(' ') || '—'],
    ['Description', esc(pkg.description)],
  ]);
  const versionsTable = table(['Version', 'Released', 'Changelog', 'Digest', 'Commit', 'Built by'], pkg.versions.slice().reverse(), (v) => W.tr([
    `<code>${esc(v.version)}</code>`, esc(v.releasedAt), esc(v.changelog), `<code>${esc(v.digest)}</code>`,
    `<code>${esc(v.provenance.commit)}</code>`, esc(v.provenance.builtBy),
  ]));
  const provenanceCard = W.card('Provenance & attribution (latest)', W.dl([
    ['Source repository', `<code>${esc(latest.provenance.sourceRepo)}</code>`],
    ['Commit', `<code>${esc(latest.provenance.commit)}</code>`],
    ['Built by', esc(latest.provenance.builtBy)], ['Signed by', esc(latest.provenance.signedBy)],
    ['Manifest digest', `<code>${esc(latest.digest)}</code>`],
  ]) + (canVerify ? W.frm('/ui/provenance/verify', [
    W.hidden('packageId', pkg.id), W.hidden('version', latest.version),
  ], { submit: 'Verify provenance', sec: true, note: 'Recomputes the manifest digest and compares it to the published digest.' }) : ''));
  const installFormHtml = canInstall ? W.card('Install to your organization', W.frm('/ui/installs/install', [
    W.hidden('packageId', pkg.id),
    W.hidden('expectedVersion', latest.version),
  ], { submit: `Install v${latest.version}` })) : '';
  const publishVersionForm = canPublishVersion ? W.card('Publish a new version (publisher)', W.frm('/ui/versions/publish', [
    W.hidden('packageId', pkg.id),
    W.fld('Version (semver)', W.txt('version', nextPatch(latest.version))),
    W.fld('Changelog', W.txt('changelog', '')),
  ], { submit: 'Publish version', note: 'Published versions are immutable; duplicate versions are rejected.' })) : '';
  const reviewForm = canReview ? W.card('Write a review', W.frm('/ui/reviews/create', [
    W.hidden('packageId', pkg.id),
    W.fld('Rating', W.sel('rating', [[5, '★★★★★ 5'], [4, '★★★★ 4'], [3, '★★★ 3'], [2, '★★ 2'], [1, '★ 1']])),
    W.fld('Review text', W.ta('text', '')),
  ], { submit: 'Post review', note: 'One review per organization per package.' })) : '';
  const reviewsTable = reviews.length ? table(['Review', 'Org', 'By', 'Rating', 'Text', 'At'], reviews, (r) => W.tr([
    `<code>${esc(r.id)}</code>`, esc(C.orgName(s, r.orgId)), esc(userName(s, r.byId)), `${r.rating}/5`, esc(r.text), `<span class="mono">${esc(r.at)}</span>`,
  ])) : '<p class="muted">No reviews yet.</p>';
  return W.page(ctx.app.config, {
    title: pkg.name, user: ctx.user, query: ctx.query,
    body: `
<h1>${esc(pkg.name)} <span class="muted">${esc(pkg.slug)}</span></h1>
${W.card('Listing overview', overview)}
${W.card('Versions (immutable, newest first)', versionsTable)}
${provenanceCard}
${installFormHtml}
${publishVersionForm}
${reviewForm}
${W.card('Reviews', reviewsTable)}`,
  });
}
function nextPatch(v) {
  const parts = String(v).split('.');
  parts[2] = String((parseInt(parts[2], 10) || 0) + 1);
  return parts.join('.');
}

function publisher(ctx) {
  const canPublish = can(ctx, 'listing:publish');
  const form = canPublish ? W.card('Publish a new listing', W.frm('/ui/listings/publish', [
    W.fld('Name', W.txt('name', '', { required: true })),
    W.fld('Slug (auto if blank)', W.txt('slug', '')),
    W.fld('Category', W.txt('category', 'finance-ops')),
    W.fld('License model', W.sel('licenseModel', [['subscription', 'Subscription'], ['per-seat', 'Per-seat'], ['free', 'Free']])),
    W.fld('Summary', W.txt('summary', '')),
    W.fld('Description', W.ta('description', '')),
    W.fld('Tags (comma-sep)', W.txt('tags', '')),
    W.fld('Source repository', W.txt('sourceRepo', 'https://git.example.internal/you/package')),
  ], { submit: 'Publish listing v1.0.0', note: 'A manifest + sha256 digest + provenance record are created with the first version.' })) : '<p class="muted">Sign in as a publisher to list packages.</p>';
  return W.page(ctx.app.config, {
    title: 'Publisher console', user: ctx.user, query: ctx.query,
    body: `<h1>Publisher console</h1>${form}
<p class="muted">Scope note: FlowMart is the store surface only. It does not define or execute workflows — packages are opaque artifacts with provenance; execution belongs to the platform consuming them.</p>`,
  });
}

function installs(ctx) {
  const s = ctx.state;
  const orgId = orgOf(ctx);
  const canManage = can(ctx, 'install:manage');
  const mine = orgId ? s.installs.filter((i) => i.orgId === orgId) : [];
  const cards = mine.map((i) => {
    const pkg = s.packages.find((p) => p.id === i.packageId);
    const latest = C.latestVersion(pkg);
    return W.card(`Install <code>${esc(i.id)}</code> — ${esc(pkg.name)} <code>${esc(i.version)}</code> ${W.statusPill(i.status)}`, `
${W.dl([
  ['Organization', esc(C.orgName(s, i.orgId))], ['Entitlement', entBadge(ctx, i.entitlementId)],
  ['Latest published', `<code>${esc(latest.version)}</code>${i.version !== latest.version ? ' — upgrade available' : ' — up to date'}`],
  ['Installed by', esc(userName(s, i.installedById))], ['Installed at', `<span class="mono">${esc(i.installedAt)}</span>`],
  ['Config', `<code>${esc(JSON.stringify(i.config))}</code>`],
])}
${table(['History'], i.history, (h) => W.tr([`${esc(h.action)} ${h.fromVersion ? esc(h.fromVersion) + ' → ' : ''}${esc(h.toVersion)} <span class="muted">by ${esc(userName(s, h.byId))} at ${esc(h.at)}</span>`]))}
${canManage ? W.frm('/ui/installs/configure', [
  W.hidden('installId', i.id),
  W.fld('Config keys to merge (JSON)', W.ta('config', '{\n  "key": "value"\n}')),
], { submit: 'Apply config', sec: true }) : ''}
${canManage && i.version !== latest.version ? W.frm('/ui/installs/upgrade', [
  W.hidden('installId', i.id),
  W.hidden('expectedCurrentVersion', i.version),
  W.fld('Target version', W.sel('targetVersion', pkg.versions.filter((v) => v.version !== i.version).map((v) => [v.version, v.version + ' — ' + v.changelog]).reverse())),
], { submit: `Upgrade to ${latest.version}` }) : ''}
${canManage && i.history.some((h) => h.fromVersion) ? W.frm('/ui/installs/rollback', [W.hidden('installId', i.id)], { submit: 'Rollback to previous', sec: true }) : ''}`);
  }).join('');
  return W.page(ctx.app.config, {
    title: 'Installs', user: ctx.user, query: ctx.query,
    body: `<h1>Your installs</h1>${cards || '<p class="muted">No installs for your organization yet.</p>'}`,
  });
}

function entitlements(ctx) {
  const s = ctx.state;
  const canGrant = can(ctx, 'entitlement:grant');
  const canRevoke = can(ctx, 'entitlement:revoke');
  const rows = s.entitlements.map((e) => W.tr([
    `<code>${esc(e.id)}</code>`, esc(C.orgName(s, e.orgId)), pkgLink(s, e.packageId), esc(e.type), e.seats || '—',
    W.statusPill(entStatus(ctx, e)), esc(e.validUntil),
    canRevoke && e.status === 'active' ? W.frm('/ui/entitlements/revoke', [
      W.hidden('entitlementId', e.id), W.fld('Reason', W.txt('reason', 'Commercial decision.')),
    ], { submit: 'Revoke' }) : '—',
  ])).join('');
  const grantForm = canGrant ? W.card('Grant an entitlement (admin)', W.frm('/ui/entitlements/grant', [
    W.fld('Organization', W.sel('orgId', s.orgs.map((o) => [o.id, o.name]))),
    W.fld('Package', W.sel('packageId', s.packages.map((p) => [p.id, `${p.name} (${p.slug})`]))),
    W.fld('Type', W.sel('type', [['subscription', 'Subscription'], ['per-seat', 'Per-seat'], ['trial', 'Trial']])),
    W.fld('Seats (per-seat only)', W.txt('seats', '10', { type: 'number', min: 1 })),
    W.fld('Valid until', W.txt('validUntil', '2027-09-12', { type: 'date' })),
  ], { submit: 'Grant' })) : '';
  return W.page(ctx.app.config, {
    title: 'Entitlements', user: ctx.user, query: ctx.query,
    body: `<h1>Entitlements &amp; licenses</h1>
${W.card('All entitlements', W.tbl(['ID', 'Org', 'Package', 'Type', 'Seats', 'Status', 'Valid until', 'Action'], rows))}
${grantForm}`,
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

module.exports = { dashboard, catalog, listing, publisher, installs, entitlements, notifications };
