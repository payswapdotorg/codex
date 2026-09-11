'use strict';
/**
 * FlowMart domain operations — catalog: publish listings, publish versions,
 * provenance verification, reviews, search. Failure switches take precedence
 * over natural record state. Packages remain opaque artifacts (see seed note).
 */

const R = require('../_lib/runtime');
const { digestOf } = require('./seed');

function find(arr, id) { return arr.find((x) => x.id === id); }
function today(ctx) { return ctx.state.meta.today || new Date().toISOString().slice(0, 10); }
function isPast(ctx, d) { return String(d || '9999-12-31') < today(ctx); }
function latestVersion(pkg) { return pkg.versions[pkg.versions.length - 1]; }

function publishListing(ctx) {
  const b = ctx.body;
  const slug = String(b.slug || b.name || '').toLowerCase().replace(/[^a-z0-9]+/g, '-').replace(/^-|-$/g, '');
  if (!b.name || !slug) throw new R.AppError(400, 'validation_error', 'Listing name and slug are required.');
  if (ctx.state.packages.find((p) => p.slug === slug)) {
    throw new R.AppError(409, 'duplicate_event', `A listing with slug "${slug}" already exists in the catalog.`, { slug });
  }
  const manifest = { name: slug, version: '1.0.0', entry: 'package.json', files: ['manifest.json'] };
  const pkg = {
    id: R.nextId(ctx.state, 'pkSeq', 'pk-'), slug, name: String(b.name), publisherId: ctx.user.id,
    category: String(b.category || 'general'), licenseModel: ['free', 'subscription', 'per-seat'].includes(b.licenseModel) ? b.licenseModel : 'subscription',
    summary: String(b.summary || ''), description: String(b.description || ''),
    tags: R.asList(b.tags),
    versions: [{
      version: '1.0.0', releasedAt: today(ctx), changelog: 'Initial release.',
      manifest, digest: digestOf(manifest),
      provenance: { sourceRepo: String(b.sourceRepo || 'https://git.example.internal/' + ctx.user.username + '/' + slug), commit: Math.random().toString(16).slice(2, 9), builtBy: ctx.user.username, signedBy: ctx.user.username },
    }],
    stats: { installs: 0, stars: 0, reviews: 0 },
  };
  ctx.state.packages.push(pkg);
  ctx.emit('listing.published', {
    subject: pkg.id, summary: `Listing "${pkg.name}" (${slug}) v1.0.0 published by ${ctx.user.name}`,
    data: { packageId: pkg.id, slug, version: '1.0.0', digest: pkg.versions[0].digest },
  });
  const n = ctx.notify('u-2', 'marketplace', `New listing: ${pkg.name}`, `${ctx.user.name} published "${pkg.name}" (${slug}).`);
  return {
    message: `Listing ${pkg.id} ("${pkg.name}") published at v1.0.0.`,
    warning: n && n.status === 'failed' ? 'Listing published, but the admin notification could not be delivered.' : null,
    redirect: `/listings/${slug}`, package: pkg,
  };
}

function publishVersion(ctx) {
  const b = ctx.body;
  const pkg = find(ctx.state.packages, b.packageId);
  if (!pkg && b.slug) {
    ctx.state.packages.find((p) => p.slug === b.slug);
  }
  const target = pkg || ctx.state.packages.find((p) => p.slug === b.slug);
  if (!target) throw new R.AppError(404, 'package_not_found', `Package "${b.packageId || b.slug}" does not exist.`);
  if (ctx.failure === 'data_conflict') {
    throw new R.AppError(409, 'data_conflict', `Another publisher session already released a version for "${target.name}" (simulated).`, { packageId: target.id, simulated: true });
  }
  const version = String(b.version || '').trim() || '1.0.1';
  if (target.versions.find((v) => v.version === version)) {
    throw new R.AppError(409, 'duplicate_event', `Version ${version} of "${target.name}" already exists — published versions are immutable.`, { packageId: target.id, version });
  }
  const prev = latestVersion(target);
  const manifest = Object.assign({}, prev.manifest, { version });
  const release = {
    version, releasedAt: today(ctx), changelog: String(b.changelog || ''),
    manifest, digest: digestOf(manifest),
    provenance: { sourceRepo: prev.provenance.sourceRepo, commit: Math.random().toString(16).slice(2, 9), builtBy: ctx.user.username, signedBy: ctx.user.username },
  };
  target.versions.push(release);
  ctx.emit('version.published', {
    subject: `${target.id}@${version}`, summary: `"${target.name}" ${version} released by ${ctx.user.name}: ${release.changelog}`,
    data: { packageId: target.id, version, digest: release.digest },
  });
  // notify org admins with active installs (upgrade available)
  const installers = ctx.state.installs.filter((i) => i.packageId === target.id && i.status === 'active');
  const warnings = [];
  for (const inst of installers) {
    const org = find(ctx.state.orgs, inst.orgId);
    const n = ctx.notify(org ? org.adminId : null, 'marketplace', `Upgrade available: ${target.name} ${version}`,
      `Your install ${inst.id} can upgrade from ${inst.version} to ${version}.`);
    if (n && n.status === 'failed') warnings.push(org ? org.name : inst.orgId);
  }
  return {
    message: `Version ${version} of "${target.name}" published (${installers.length} installer(s) notified).`,
    warning: warnings.length ? `Released, but upgrade notifications failed for: ${warnings.join(', ')}.` : null,
    redirect: `/listings/${target.slug}`, release,
  };
}

function verifyProvenance(ctx) {
  const b = ctx.body;
  const pkg = find(ctx.state.packages, b.packageId) || ctx.state.packages.find((p) => p.slug === b.slug);
  if (!pkg) throw new R.AppError(404, 'package_not_found', `Package "${b.packageId || b.slug}" does not exist.`);
  const version = String(b.version || latestVersion(pkg).version);
  const rel = pkg.versions.find((v) => v.version === version);
  if (!rel) throw new R.AppError(404, 'version_not_found', `Version "${version}" of "${pkg.name}" does not exist.`, { packageId: pkg.id, version });
  if (ctx.failure === 'data_conflict') {
    throw new R.AppError(409, 'data_conflict',
      `Provenance mismatch (simulated): recomputed digest for "${pkg.name}"@${version} does not match the published digest.`,
      { packageId: pkg.id, version, publishedDigest: rel.digest, simulated: true });
  }
  const recomputed = digestOf(rel.manifest);
  const match = recomputed === rel.digest;
  ctx.emit('provenance.verified', {
    subject: `${pkg.id}@${version}`,
    summary: `Provenance verification of "${pkg.name}"@${version}: ${match ? 'MATCH' : 'MISMATCH'} (${recomputed})`,
    data: { packageId: pkg.id, version, match, digest: recomputed, commit: rel.provenance.commit, builtBy: rel.provenance.builtBy },
  });
  if (!match) {
    throw new R.AppError(409, 'data_conflict', `Provenance mismatch: recomputed digest ${recomputed} != published ${rel.digest}.`, { packageId: pkg.id, version });
  }
  return {
    message: `Provenance verified for "${pkg.name}"@${version}: digest ${rel.digest} matches the published manifest.`,
    match: true, package: { id: pkg.id, name: pkg.name, slug: pkg.slug }, version: rel.version,
    digest: rel.digest, provenance: rel.provenance,
  };
}

function writeReview(ctx) {
  const b = ctx.body;
  const pkg = find(ctx.state.packages, b.packageId) || ctx.state.packages.find((p) => p.slug === b.slug);
  if (!pkg) throw new R.AppError(404, 'package_not_found', `Package "${b.packageId || b.slug}" does not exist.`);
  const orgId = ctx.user.orgId || orgForUser(ctx.state, ctx.user.id);
  if (!orgId) throw new R.AppError(403, 'permission_denied', 'Reviews must come from an organization member; your user is not in any org.', { user: ctx.user.username });
  if (ctx.state.reviews.find((r) => r.packageId === pkg.id && r.orgId === orgId)) {
    throw new R.AppError(409, 'duplicate_event', `Your organization already reviewed "${pkg.name}" — one review per org per package.`, { packageId: pkg.id, orgId });
  }
  const rating = Math.max(1, Math.min(5, parseInt(b.rating, 10) || 5));
  const review = {
    id: R.nextId(ctx.state, 'revSeq', 'rv-'), packageId: pkg.id, orgId, byId: ctx.user.id,
    rating, text: String(b.text || ''), at: new Date().toISOString(),
  };
  ctx.state.reviews.push(review);
  pkg.stats.reviews += 1;
  const total = ctx.state.reviews.filter((r) => r.packageId === pkg.id).reduce((a, r) => a + r.rating, 0);
  pkg.stats.stars = Math.round((total / pkg.stats.reviews) * 10) / 10;
  ctx.emit('review.posted', {
    subject: review.id, summary: `${orgName(ctx.state, orgId)} rated "${pkg.name}" ${rating}/5`,
    data: { reviewId: review.id, packageId: pkg.id, rating },
  });
  const n = ctx.notify(pkg.publisherId, 'marketplace', `New ${rating}★ review on ${pkg.name}`, `${orgName(ctx.state, orgId)}: "${review.text.slice(0, 90)}"`);
  return {
    message: `Review ${review.id} posted (${rating}/5).`,
    warning: n && n.status === 'failed' ? 'Review posted, but the publisher notification could not be delivered.' : null,
    redirect: `/listings/${pkg.slug}`, review,
  };
}
function orgForUser(state, userId) {
  const org = state.orgs.find((o) => o.memberIds.includes(userId));
  return org ? org.id : null;
}
function orgName(state, orgId) {
  const o = (state.orgs || []).find((x) => x.id === orgId);
  return o ? o.name : orgId;
}

function searchCatalog(ctx) {
  const q = String(ctx.query.get('q') || '').toLowerCase();
  const category = ctx.query.get('category') || null;
  const tag = ctx.query.get('tag') || null;
  let results = ctx.state.packages.slice();
  if (category) results = results.filter((p) => p.category === category);
  if (tag) results = results.filter((p) => p.tags.includes(tag));
  if (q) {
    results = results.map((p) => {
      const hay = `${p.name} ${p.slug} ${p.summary} ${p.description} ${p.tags.join(' ')}`.toLowerCase();
      let score = 0;
      for (const term of q.split(/\s+/).filter(Boolean)) {
        if (p.name.toLowerCase().includes(term)) score += 3;
        if (p.slug.includes(term)) score += 2;
        if (hay.includes(term)) score += 1;
      }
      return { p, score };
    }).filter((x) => x.score > 0).sort((a, b) => b.score - a.score).map((x) => x.p);
  }
  return { ok: true, query: { q, category, tag }, count: results.length, results: results.map(listingSummary) };
}
function listingSummary(p) {
  const latest = latestVersion(p);
  return {
    id: p.id, slug: p.slug, name: p.name, category: p.category, summary: p.summary,
    tags: p.tags, licenseModel: p.licenseModel, latestVersion: latest.version,
    latestDigest: latest.digest, stats: p.stats, publisher: p.publisherId,
  };
}

module.exports = { publishListing, publishVersion, verifyProvenance, writeReview, searchCatalog, listingSummary, latestVersion, find, today, isPast, orgForUser, orgName };
