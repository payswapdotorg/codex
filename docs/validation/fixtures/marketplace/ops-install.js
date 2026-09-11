'use strict';
/**
 * FlowMart domain operations — installs, configuration, upgrades, rollback,
 * entitlements (the commercial boundary of the marketplace).
 */

const R = require('../_lib/runtime');
const C = require('./ops');
const { find, today, isPast, orgForUser, orgName, latestVersion } = C;

function entitlementFor(state, orgId, packageId) {
  return state.entitlements.find((e) => e.orgId === orgId && e.packageId === packageId) || null;
}
function checkEntitlement(ctx, orgId, pkg, action) {
  const ent = entitlementFor(ctx.state, orgId, pkg.id);
  if (ctx.failure === 'stale_entitlement') {
    throw new R.AppError(409, 'stale_entitlement',
      `Entitlement for "${pkg.name}" is stale/revoked (simulated): ${action} is blocked for ${orgName(ctx.state, orgId)}.`,
      { packageId: pkg.id, orgId, simulated: true });
  }
  if (!ent) return null; // callers decide: install auto-grants a trial
  if (ent.status === 'revoked' || isPast(ctx, ent.validUntil)) {
    throw new R.AppError(409, 'stale_entitlement',
      `Entitlement ${ent.id} for "${pkg.name}" (${orgName(ctx.state, orgId)}) is ${ent.status === 'revoked' ? 'revoked' : 'expired (valid until ' + ent.validUntil + ')'} — ${action} requires an active license.`,
      { entitlementId: ent.id, packageId: pkg.id, orgId, status: ent.status, validUntil: ent.validUntil });
  }
  return ent;
}

function installPackage(ctx) {
  const b = ctx.body;
  const pkg = find(ctx.state.packages, b.packageId) || ctx.state.packages.find((p) => p.slug === b.slug);
  if (!pkg) throw new R.AppError(404, 'package_not_found', `Package "${b.packageId || b.slug}" does not exist.`);
  const orgId = orgForUser(ctx.state, ctx.user.id);
  if (!orgId) throw new R.AppError(403, 'permission_denied', 'Your user is not an admin/member of an organization; installs belong to orgs.', { user: ctx.user.username });
  const latest = latestVersion(pkg);
  if (ctx.failure === 'missing_asset') {
    throw new R.AppError(404, 'missing_asset',
      `Package archive for "${pkg.name}"@${b.expectedVersion || latest.version} could not be resolved in the registry blob store (simulated dangling reference).`,
      { packageId: pkg.id, version: b.expectedVersion || latest.version, simulated: true });
  }
  const ent = checkEntitlement(ctx, orgId, pkg, 'installation');
  const existing = ctx.state.installs.find((i) => i.orgId === orgId && i.packageId === pkg.id && i.status === 'active');
  if (existing && ctx.failure !== 'data_conflict') {
    throw new R.AppError(409, 'duplicate_event',
      `${orgName(ctx.state, orgId)} already has an active install (${existing.id}, v${existing.version}) of "${pkg.name}" — upgrade it instead of installing again.`,
      { installId: existing.id, packageId: pkg.id });
  }
  if (ctx.failure === 'data_conflict') {
    throw new R.AppError(409, 'data_conflict',
      `The listing moved while you were installing (simulated): expected version ${b.expectedVersion || latest.version} is no longer latest.`,
      { packageId: pkg.id, currentVersion: latest.version, simulated: true });
  }
  if (b.expectedVersion && String(b.expectedVersion) !== latest.version) {
    throw new R.AppError(409, 'data_conflict',
      `Version mismatch: you selected ${b.expectedVersion} but the latest published version is ${latest.version}.`,
      { packageId: pkg.id, currentVersion: latest.version, yourVersion: String(b.expectedVersion) });
  }
  let entitlement = ent;
  if (!entitlement) {
    // Auto-grant a 30-day trial entitlement (documented behavior for first install).
    entitlement = {
      id: R.nextId(ctx.state, 'entSeq', 'ent-'), orgId, packageId: pkg.id, type: 'trial', seats: null,
      status: 'active', validUntil: plusDays(today(ctx), 30), grantedAt: today(ctx), grantedById: null,
    };
    ctx.state.entitlements.push(entitlement);
    ctx.emit('entitlement.granted', {
      subject: entitlement.id, summary: `Trial entitlement for "${pkg.name}" auto-granted to ${orgName(ctx.state, orgId)} (until ${entitlement.validUntil})`,
      data: { entitlementId: entitlement.id, orgId, packageId: pkg.id },
    });
  }
  const install = {
    id: R.nextId(ctx.state, 'insSeq', 'ins-'), orgId, packageId: pkg.id,
    version: latest.version, status: 'active',
    config: typeof b.config === 'object' && b.config ? b.config : {},
    installedById: ctx.user.id, installedAt: new Date().toISOString(),
    entitlementId: entitlement.id,
    history: [{ action: 'installed', fromVersion: null, toVersion: latest.version, at: new Date().toISOString(), byId: ctx.user.id }],
  };
  ctx.state.installs.push(install);
  pkg.stats.installs += 1;
  ctx.emit('package.installed', {
    subject: install.id, summary: `${orgName(ctx.state, orgId)} installed "${pkg.name}" v${latest.version} (${install.id})`,
    data: { installId: install.id, orgId, packageId: pkg.id, version: latest.version, digest: latest.digest, entitlementId: entitlement.id },
  });
  const n = ctx.notify(pkg.publisherId, 'marketplace', `${orgName(ctx.state, orgId)} installed ${pkg.name}`,
    `Install ${install.id} at v${latest.version}. Entitlement ${entitlement.id} (${entitlement.type}).`);
  return {
    message: `Installed "${pkg.name}" v${latest.version} for ${orgName(ctx.state, orgId)} (install ${install.id}, entitlement ${entitlement.id}).`,
    warning: n && n.status === 'failed' ? 'Install recorded, but the publisher notification could not be delivered.' : null,
    redirect: '/installs', install, entitlement,
  };
}
function plusDays(dateStr, days) {
  const d = new Date(dateStr + 'T00:00:00Z');
  d.setUTCDate(d.getUTCDate() + days);
  return d.toISOString().slice(0, 10);
}

function configureInstall(ctx) {
  const b = ctx.body;
  const inst = find(ctx.state.installs, b.installId);
  if (!inst) throw new R.AppError(404, 'install_not_found', `Install "${b.installId}" does not exist.`);
  const pkg = find(ctx.state.packages, inst.packageId);
  const ent = checkEntitlement(ctx, inst.orgId, pkg, 'configuration');
  if (!ent) throw new R.AppError(409, 'stale_entitlement', `Install ${inst.id} has no active entitlement — configuration is locked.`, { installId: inst.id });
  const patch = typeof b.config === 'object' && b.config ? b.config : {};
  inst.config = Object.assign({}, inst.config, patch);
  inst.history.push({ action: 'configured', fromVersion: inst.version, toVersion: inst.version, at: new Date().toISOString(), byId: ctx.user.id, configKeys: Object.keys(patch) });
  ctx.emit('install.configured', {
    subject: inst.id, summary: `Install ${inst.id} (${pkg.name} v${inst.version}) reconfigured for ${orgName(ctx.state, inst.orgId)}`,
    data: { installId: inst.id, packageId: pkg.id, keys: Object.keys(patch) },
  });
  return { message: `Install ${inst.id} configured (${Object.keys(patch).join(', ') || 'no keys'}).`, redirect: '/installs', install: inst };
}

function upgradeInstall(ctx) {
  const b = ctx.body;
  const inst = find(ctx.state.installs, b.installId);
  if (!inst) throw new R.AppError(404, 'install_not_found', `Install "${b.installId}" does not exist.`);
  const pkg = find(ctx.state.packages, inst.packageId);
  const latest = latestVersion(pkg);
  const targetVersion = String(b.targetVersion || latest.version);
  const ent = checkEntitlement(ctx, inst.orgId, pkg, 'upgrade');
  if (!ent) throw new R.AppError(409, 'stale_entitlement', `Install ${inst.id} has no active entitlement — upgrades are locked.`, { installId: inst.id });
  if (ctx.failure === 'data_conflict') {
    throw new R.AppError(409, 'data_conflict', `Install ${inst.id} was upgraded by another admin while you were upgrading (simulated).`, { installId: inst.id, simulated: true });
  }
  if (!Number.isNaN(parseInt(b.expectedCurrentVersion, 10)) && String(b.expectedCurrentVersion) !== inst.version) {
    throw new R.AppError(409, 'data_conflict',
      `Install ${inst.id} moved: you expected v${b.expectedCurrentVersion} but it is at v${inst.version}.`,
      { installId: inst.id, currentVersion: inst.version });
  }
  const rel = pkg.versions.find((v) => v.version === targetVersion);
  if (!rel) throw new R.AppError(404, 'version_not_found', `Version "${targetVersion}" of "${pkg.name}" does not exist.`, { installId: inst.id, version: targetVersion });
  if (targetVersion === inst.version) {
    throw new R.AppError(409, 'duplicate_event', `Install ${inst.id} is already at v${targetVersion}.`, { installId: inst.id });
  }
  const from = inst.version;
  inst.version = targetVersion;
  inst.history.push({ action: 'upgraded', fromVersion: from, toVersion: targetVersion, at: new Date().toISOString(), byId: ctx.user.id });
  ctx.emit('install.upgraded', {
    subject: inst.id, summary: `${orgName(ctx.state, inst.orgId)} upgraded "${pkg.name}" ${from} → ${targetVersion}`,
    data: { installId: inst.id, packageId: pkg.id, fromVersion: from, toVersion: targetVersion, digest: rel.digest },
  });
  const org = find(ctx.state.orgs, inst.orgId);
  const n = ctx.notify(org ? org.adminId : null, 'marketplace', `${pkg.name} upgraded to ${targetVersion}`,
    `Install ${inst.id} moved from ${from} to ${targetVersion}.`);
  return {
    message: `Install ${inst.id} upgraded ${from} → ${targetVersion}.`,
    warning: n && n.status === 'failed' ? 'Upgrade recorded, but the org-admin notification could not be delivered.' : null,
    redirect: '/installs', install: inst,
  };
}

function rollbackInstall(ctx) {
  const b = ctx.body;
  const inst = find(ctx.state.installs, b.installId);
  if (!inst) throw new R.AppError(404, 'install_not_found', `Install "${b.installId}" does not exist.`);
  const pkg = find(ctx.state.packages, inst.packageId);
  const ent = checkEntitlement(ctx, inst.orgId, pkg, 'rollback');
  if (!ent) throw new R.AppError(409, 'stale_entitlement', `Install ${inst.id} has no active entitlement — rollback is locked.`, { installId: inst.id });
  if (ctx.failure === 'data_conflict') {
    throw new R.AppError(409, 'data_conflict', `Install ${inst.id} was rolled back by another admin concurrently (simulated).`, { installId: inst.id, simulated: true });
  }
  const prevEntry = [...inst.history].reverse().find((h) => h.toVersion && h.toVersion !== inst.version);
  const prevVersion = prevEntry ? prevEntry.toVersion : null;
  if (!prevVersion) {
    throw new R.AppError(409, 'data_conflict', `Install ${inst.id} has no prior version to roll back to (installed at v${inst.version}, never upgraded).`, { installId: inst.id });
  }
  const from = inst.version;
  inst.version = prevVersion;
  inst.history.push({ action: 'rolled_back', fromVersion: from, toVersion: prevVersion, at: new Date().toISOString(), byId: ctx.user.id });
  ctx.emit('install.rolled_back', {
    subject: inst.id, summary: `${orgName(ctx.state, inst.orgId)} rolled back "${pkg.name}" ${from} → ${prevVersion}`,
    data: { installId: inst.id, packageId: pkg.id, fromVersion: from, toVersion: prevVersion },
  });
  const org = find(ctx.state.orgs, inst.orgId);
  const n = ctx.notify(org ? org.adminId : null, 'marketplace', `${pkg.name} rolled back to ${prevVersion}`,
    `Install ${inst.id} rolled back from ${from} to ${prevVersion}.`);
  return {
    message: `Install ${inst.id} rolled back ${from} → ${prevVersion}.`,
    warning: n && n.status === 'failed' ? 'Rollback recorded, but the org-admin notification could not be delivered.' : null,
    redirect: '/installs', install: inst,
  };
}

function revokeEntitlement(ctx) {
  const b = ctx.body;
  const ent = find(ctx.state.entitlements, b.entitlementId);
  if (!ent) throw new R.AppError(404, 'entitlement_not_found', `Entitlement "${b.entitlementId}" does not exist.`);
  if (ctx.failure === 'data_conflict') {
    throw new R.AppError(409, 'data_conflict', `Entitlement ${ent.id} was already revoked (simulated).`, { entitlementId: ent.id, simulated: true });
  }
  if (ent.status === 'revoked') {
    throw new R.AppError(409, 'data_conflict', `Entitlement ${ent.id} is already revoked.`, { entitlementId: ent.id });
  }
  ent.status = 'revoked';
  ent.revokedAt = new Date().toISOString();
  ent.revokedById = ctx.user.id;
  ent.revokeReason = String(b.reason || 'revoked by operator');
  const pkg = find(ctx.state.packages, ent.packageId);
  ctx.emit('entitlement.revoked', {
    subject: ent.id, summary: `Entitlement ${ent.id} for "${pkg ? pkg.name : ent.packageId}" revoked by ${ctx.user.name}: ${ent.revokeReason}`,
    data: { entitlementId: ent.id, orgId: ent.orgId, packageId: ent.packageId },
  });
  const org = find(ctx.state.orgs, ent.orgId);
  const n = ctx.notify(org ? org.adminId : null, 'marketplace', `Your entitlement for ${pkg ? pkg.name : ent.packageId} was revoked`,
    `Entitlement ${ent.id} revoked: ${ent.revokeReason}. Installs are now locked (stale entitlement).`);
  return {
    message: `Entitlement ${ent.id} revoked.`,
    warning: n && n.status === 'failed' ? 'Revocation recorded, but the org-admin notification could not be delivered.' : null,
    redirect: '/entitlements', entitlement: ent,
  };
}

function grantEntitlement(ctx) {
  const b = ctx.body;
  const org = find(ctx.state.orgs, b.orgId);
  if (!org) throw new R.AppError(404, 'org_not_found', `Organization "${b.orgId}" does not exist.`);
  const pkg = find(ctx.state.packages, b.packageId) || ctx.state.packages.find((p) => p.slug === b.slug);
  if (!pkg) throw new R.AppError(404, 'package_not_found', `Package "${b.packageId || b.slug}" does not exist.`);
  const type = ['subscription', 'per-seat', 'trial'].includes(b.type) ? b.type : 'subscription';
  const ent = {
    id: R.nextId(ctx.state, 'entSeq', 'ent-'), orgId: org.id, packageId: pkg.id, type,
    seats: type === 'per-seat' ? (parseInt(b.seats, 10) || 10) : null,
    status: 'active', validUntil: String(b.validUntil || plusDays(today(ctx), 365)),
    grantedAt: today(ctx), grantedById: ctx.user.id,
  };
  ctx.state.entitlements.push(ent);
  ctx.emit('entitlement.granted', {
    subject: ent.id, summary: `Entitlement ${ent.id} (${type}) for "${pkg.name}" granted to ${org.name} by ${ctx.user.name} (until ${ent.validUntil})`,
    data: { entitlementId: ent.id, orgId: org.id, packageId: pkg.id, type, validUntil: ent.validUntil },
  });
  const n = ctx.notify(org.adminId, 'marketplace', `Your org now has a ${type} license for ${pkg.name}`,
    `Entitlement ${ent.id} active until ${ent.validUntil}.`);
  return {
    message: `Entitlement ${ent.id} granted to ${org.name}.`,
    warning: n && n.status === 'failed' ? 'Entitlement granted, but the org-admin notification could not be delivered.' : null,
    redirect: '/entitlements', entitlement: ent,
  };
}

module.exports = { installPackage, configureInstall, upgradeInstall, rollbackInstall, revokeEntitlement, grantEntitlement, entitlementFor, plusDays };
