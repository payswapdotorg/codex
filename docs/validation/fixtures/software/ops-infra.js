'use strict';
/**
 * ForgeOps domain operations — deployments (promote/rollback), incidents, docs.
 * The deployment console's promote step is the production human-approval gate.
 */

const R = require('../_lib/runtime');
const { find } = require('./ops');

function today(ctx) { return ctx.state.meta.today || new Date().toISOString().slice(0, 10); }

function promoteDeployment(ctx) {
  const b = ctx.body;
  const dep = find(ctx.state.deployments, b.deploymentId);
  if (!dep) throw new R.AppError(404, 'deployment_not_found', `Deployment "${b.deploymentId}" does not exist.`);
  // Failure switches take precedence over natural record state so switch behavior is deterministic.
  if (ctx.failure === 'data_conflict') {
    throw new R.AppError(409, 'data_conflict', `Deployment ${dep.id} was already promoted by another release manager (simulated).`, { deploymentId: dep.id, simulated: true });
  }
  if (ctx.failure === 'stale_entitlement') {
    throw new R.AppError(409, 'stale_entitlement',
      'Production release entitlement is stale/expired (simulated): promotion to production is blocked.',
      { entitlement: 'release-license', simulated: true });
  }
  if (ctx.failure === 'missing_asset') {
    throw new R.AppError(404, 'missing_asset',
      `Release artifact "${dep.artifactId}" could not be resolved in the artifact registry (simulated dangling reference).`,
      { artifactId: dep.artifactId, simulated: true });
  }
  if (dep.env !== 'staging' || dep.status !== 'succeeded') {
    throw new R.AppError(409, 'data_conflict', `Deployment ${dep.id} is env "${dep.env}" / status "${dep.status}" — only successful staging deployments can be promoted.`, { deploymentId: dep.id, env: dep.env, status: dep.status });
  }
  const artifact = find(ctx.state.artifacts, dep.artifactId);
  if (!artifact) {
    throw new R.AppError(404, 'missing_asset',
      `Release artifact "${dep.artifactId}" could not be resolved in the artifact registry.`,
      { artifactId: dep.artifactId });
  }
  const repo = find(ctx.state.repos, dep.repoId);
  const prodDep = {
    id: R.nextId(ctx.state, 'depSeq', 'dep-'), repoId: repo.id, prId: dep.prId,
    env: 'production', version: dep.version, status: 'succeeded', artifactId: dep.artifactId,
    at: new Date().toISOString(), promotedFromDeploymentId: dep.id,
  };
  ctx.state.deployments.push(prodDep);
  dep.status = 'promoted'; dep.promotedById = ctx.user.id;
  repo.productionVersion = dep.version;
  const target = ctx.state.services.find((s) => s.name === repo.serviceName) || null;
  if (target) target.version = dep.version;
  ctx.emit('deployment.promoted', {
    subject: prodDep.id, summary: `${repo.name} ${dep.version} promoted to production by ${ctx.user.name} (from ${dep.id})`,
    data: { deploymentId: prodDep.id, fromDeploymentId: dep.id, repoId: repo.id, version: dep.version, artifactId: dep.artifactId },
  });
  if (target) {
    ctx.emit('service.version_bumped', {
      subject: target.id, summary: `Service ${target.name} now serving version ${dep.version}`,
      data: { serviceId: target.id, version: dep.version },
    });
  }
  const team = find(ctx.state.teams, repo.ownerTeamId);
  const n = team ? ctx.notify(team.memberIds[0], 'deploy', `${repo.name} ${dep.version} is live in production`,
    `Promoted by ${ctx.user.name} from staging deployment ${dep.id}.`) : null;
  return {
    message: `${repo.name} ${dep.version} promoted to production (deployment ${prodDep.id}).`,
    warning: n && n.status === 'failed' ? 'Promotion completed, but the team notification could not be delivered.' : null,
    redirect: '/deployments', deployment: prodDep,
  };
}

function rollbackDeployment(ctx) {
  const b = ctx.body;
  const dep = find(ctx.state.deployments, b.deploymentId);
  if (!dep) throw new R.AppError(404, 'deployment_not_found', `Deployment "${b.deploymentId}" does not exist.`);
  if (ctx.failure === 'data_conflict') {
    throw new R.AppError(409, 'data_conflict', `Rollback rejected (simulated): another operator is mid-rollback for ${repoName(ctx, dep)}.`, { deploymentId: dep.id, simulated: true });
  }
  if (dep.env !== 'production') {
    throw new R.AppError(409, 'data_conflict', `Deployment ${dep.id} is env "${dep.env}" — only production deployments can be rolled back.`, { deploymentId: dep.id, env: dep.env });
  }
  const repo = find(ctx.state.repos, dep.repoId);
  const prior = ctx.state.deployments.filter((d) => d.repoId === repo.id && d.env === 'production' && d.id !== dep.id && d.status === 'succeeded' && d.version !== dep.version)
    .sort((a, b2) => (a.at < b2.at ? 1 : -1))[0];
  if (!prior) {
    throw new R.AppError(409, 'data_conflict', `No prior production deployment exists for ${repo.name} — nothing to roll back to.`, { deploymentId: dep.id });
  }
  const rb = {
    id: R.nextId(ctx.state, 'depSeq', 'dep-'), repoId: repo.id, prId: prior.prId,
    env: 'production', version: prior.version, status: 'succeeded', artifactId: prior.artifactId,
    at: new Date().toISOString(), rollbackOf: dep.id,
  };
  dep.status = 'rolled_back';
  ctx.state.deployments.push(rb);
  repo.productionVersion = prior.version;
  const svc = ctx.state.services.find((s) => s.name === repo.serviceName);
  if (svc) svc.version = prior.version;
  ctx.emit('deployment.rolled_back', {
    subject: rb.id, summary: `${repo.name} rolled back from ${dep.version} to ${prior.version} by ${ctx.user.name}`,
    data: { deploymentId: rb.id, rollbackOf: dep.id, repoId: repo.id, fromVersion: dep.version, toVersion: prior.version },
  });
  const n = ctx.notify('u-4', 'deploy', `${repo.name} rolled back to ${prior.version}`,
    `${ctx.user.name} rolled back production after ${dep.id}.`);
  return {
    message: `${repo.name} rolled back to ${prior.version} (deployment ${rb.id}).`,
    warning: n && n.status === 'failed' ? 'Rollback completed, but the on-call notification could not be delivered.' : null,
    redirect: '/deployments', deployment: rb,
  };
}
function repoName(ctx, dep) {
  const r = find(ctx.state.repos, dep.repoId);
  return r ? r.name : dep.repoId;
}

function openIncident(ctx) {
  const b = ctx.body;
  const service = find(ctx.state.services, b.serviceId);
  if (!service) throw new R.AppError(404, 'service_not_found', `Service "${b.serviceId}" does not exist.`);
  const sev = ['sev1', 'sev2', 'sev3'].includes(b.severity) ? b.severity : 'sev3';
  const inc = {
    id: R.nextId(ctx.state, 'incSeq', 'inc-'), severity: sev, title: String(b.title || 'Untitled incident'),
    serviceId: service.id, status: 'open', reporterId: ctx.user.id, assigneeId: ctx.user.id,
    openedAt: new Date().toISOString(), resolvedAt: null,
    description: String(b.description || ''), timeline: [{ at: new Date().toISOString(), byId: ctx.user.id, text: 'Opened from the incident console.' }],
  };
  ctx.state.incidents.push(inc);
  service.status = sev === 'sev3' ? 'degraded' : 'down';
  ctx.emit('incident.opened', {
    subject: inc.id, summary: `${sev} incident "${inc.title}" opened on ${service.name} by ${ctx.user.name}`,
    data: { incidentId: inc.id, serviceId: service.id, severity: sev },
  });
  const n = ctx.notify('u-4', 'incident', `${sev}: ${inc.title}`,
    `${ctx.user.name} opened ${sev} on ${service.name}. You are on-call.`);
  return {
    message: `Incident ${inc.id} opened (${sev}); ${service.name} marked ${service.status}.`,
    warning: n && n.status === 'failed' ? 'Incident opened, but the on-call page could not be delivered.' : null,
    redirect: '/incidents', incident: inc,
  };
}

function resolveIncident(ctx) {
  const b = ctx.body;
  const inc = find(ctx.state.incidents, b.incidentId);
  if (!inc) throw new R.AppError(404, 'incident_not_found', `Incident "${b.incidentId}" does not exist.`);
  if (ctx.failure === 'data_conflict') {
    throw new R.AppError(409, 'data_conflict', `Incident ${inc.id} was already resolved by another responder (simulated).`, { incidentId: inc.id, simulated: true });
  }
  if (inc.status === 'resolved') {
    throw new R.AppError(409, 'data_conflict', `Incident ${inc.id} is already resolved.`, { incidentId: inc.id });
  }
  inc.status = 'resolved';
  inc.resolvedAt = new Date().toISOString();
  inc.timeline.push({ at: inc.resolvedAt, byId: ctx.user.id, text: String(b.resolution || 'Resolved.') });
  const service = find(ctx.state.services, inc.serviceId);
  if (service && service.status !== 'healthy' && !ctx.state.incidents.some((i) => i.serviceId === service.id && i.status !== 'resolved')) {
    service.status = 'healthy';
  }
  ctx.emit('incident.resolved', {
    subject: inc.id, summary: `Incident ${inc.id} "${inc.title}" resolved by ${ctx.user.name}`,
    data: { incidentId: inc.id, serviceId: inc.serviceId },
  });
  const n = ctx.notify(inc.reporterId, 'incident', `Incident ${inc.id} resolved`, `${ctx.user.name}: ${b.resolution || 'Resolved.'}`);
  return {
    message: `Incident ${inc.id} resolved; ${service ? service.name + ' is ' + service.status : 'service updated'}.`,
    warning: n && n.status === 'failed' ? 'Incident resolved, but the reporter notification could not be delivered.' : null,
    redirect: '/incidents', incident: inc,
  };
}

function updateDoc(ctx) {
  const b = ctx.body;
  const doc = find(ctx.state.docs, b.docId);
  if (!doc) throw new R.AppError(404, 'doc_not_found', `Document "${b.docId}" does not exist.`);
  if (ctx.failure === 'data_conflict') {
    throw new R.AppError(409, 'data_conflict', `Document ${doc.id} was edited by someone else while you were saving (simulated optimistic-concurrency mismatch).`, { docId: doc.id, currentVersion: doc.version, simulated: true });
  }
  const expected = parseInt(b.expectedVersion, 10);
  if (!Number.isNaN(expected) && expected !== doc.version) {
    throw new R.AppError(409, 'data_conflict',
      `Version mismatch: you edited v${expected} but the current document is v${doc.version} — reload and reapply.`,
      { docId: doc.id, currentVersion: doc.version, yourVersion: expected });
  }
  if (b.body !== undefined) doc.body = String(b.body);
  doc.version += 1;
  ctx.emit('doc.updated', {
    subject: doc.id, summary: `Doc "${doc.title}" updated to v${doc.version} by ${ctx.user.name}`,
    data: { docId: doc.id, version: doc.version },
  });
  return { message: `Document "${doc.title}" saved as v${doc.version}.`, redirect: `/docs/${doc.id}`, doc };
}

module.exports = { promoteDeployment, rollbackDeployment, openIncident, resolveIncident, updateDoc, today };
