'use strict';
/**
 * SiteBuild domain operations. Each mutation is exposed BOTH as a JSON API endpoint
 * and as an HTML form (same handler), so the human path and the API path produce
 * identical state changes and events. Ops return { message, warning?, redirect?, ...payload }.
 * The fixture clock lives in state.meta.today (the demo world believes it is that date).
 */

const R = require('../_lib/runtime');
const W = require('../_lib/web');

function today(ctx) { return ctx.state.meta.today || new Date().toISOString().slice(0, 10); }
function isPast(ctx, dateStr) { return String(dateStr || '') < today(ctx); }
function find(arr, id) { return arr.find((x) => x.id === id); }

// ---------------------------------------------------------------- progress

function submitProgress(ctx) {
  const b = ctx.body;
  const project = find(ctx.state.projects, b.projectId);
  if (!project) throw new R.AppError(404, 'project_not_found', `Project "${b.projectId}" does not exist.`);
  const date = String(b.reportDate || today(ctx)).slice(0, 10);
  if (ctx.state.progressReports.find((r) => r.projectId === project.id && r.reportDate === date)) {
    throw new R.AppError(409, 'duplicate_event',
      `A progress report for ${project.id} on ${date} already exists — daily reports are one per project per day.`,
      { projectId: project.id, reportDate: date });
  }
  const taskIds = R.asList(b.taskIds).filter((t) => find(ctx.state.tasks, t) && find(ctx.state.tasks, t).projectId === project.id);
  const photoIds = R.asList(b.photoIds);
  if (ctx.failure === 'missing_asset') {
    throw new R.AppError(404, 'missing_asset',
      `Photo asset "${photoIds[0] || 'a-unknown'}" could not be resolved in the photo store (simulated dangling reference).`,
      { assetId: photoIds[0] || null, simulated: true });
  }
  for (const pid of photoIds) {
    const a = find(ctx.state.assets, pid);
    if (!a || a.kind !== 'photo') throw new R.AppError(404, 'missing_asset', `Photo asset "${pid}" does not exist in the photo store.`, { assetId: pid });
  }
  const report = {
    id: R.nextId(ctx.state, 'progressSeq', 'pr-'),
    projectId: project.id, reportDate: date, authorId: ctx.user.id,
    taskIds, percentComplete: Math.max(0, Math.min(100, parseInt(b.percentComplete, 10) || project.percentComplete)),
    notes: String(b.notes || ''), photoIds,
  };
  ctx.state.progressReports.push(report);
  for (const t of taskIds) { const task = find(ctx.state.tasks, t); if (task) task.status = 'done'; }
  if (report.percentComplete > project.percentComplete) project.percentComplete = report.percentComplete;

  ctx.emit('progress.submitted', {
    subject: project.id,
    summary: `Daily progress ${report.reportDate} for ${project.name}: ${report.percentComplete}% (${taskIds.length} tasks, ${photoIds.length} photos)`,
    data: { reportId: report.id, projectId: project.id, percentComplete: report.percentComplete, taskIds, photoIds },
  });
  const n = ctx.notify(project.managerId, 'progress', `Daily progress ${report.reportDate} — ${project.name}`,
    `${ctx.user.name} reported ${report.percentComplete}% complete. Tasks: ${taskIds.join(', ') || 'none'}.`);
  return {
    message: `Progress report ${report.id} saved for ${project.name} (${report.percentComplete}%).`,
    warning: n && n.status === 'failed' ? `Saved, but the notification to the project manager could not be delivered (check /notifications and the event feed for notification.failed).` : null,
    redirect: `/projects/${project.id}`, report, notification: n,
  };
}

function uploadPhoto(ctx) {
  const b = ctx.body;
  const project = find(ctx.state.projects, b.projectId);
  if (!project) throw new R.AppError(404, 'project_not_found', `Project "${b.projectId}" does not exist.`);
  const asset = {
    id: R.nextId(ctx.state, 'assetSeq', 'a-'),
    projectId: project.id, kind: 'photo', name: String(b.name || 'untitled.jpg'),
    sizeKb: parseInt(b.sizeKb, 10) || 900, caption: String(b.caption || ''), checksum: Math.random().toString(16).slice(2, 10),
    uploadedBy: ctx.user.id,
  };
  ctx.state.assets.push(asset);
  ctx.emit('asset.uploaded', {
    subject: asset.id, summary: `Photo "${asset.name}" attached to ${project.name} (${asset.sizeKb} kb)`,
    data: { assetId: asset.id, projectId: project.id },
  });
  return { message: `Photo ${asset.id} ("${asset.name}") stored for ${project.name}.`, redirect: `/projects/${project.id}`, asset };
}

// ---------------------------------------------------------------- procurement

function decidePurchaseRequest(ctx) {
  const b = ctx.body;
  const pr = find(ctx.state.purchaseRequests, b.requestId);
  if (!pr) throw new R.AppError(404, 'pr_not_found', `Purchase request "${b.requestId}" does not exist.`);
  const vendor = find(ctx.state.contractors, pr.vendorId);
  const decision = b.decision === 'reject' ? 'reject' : 'approve';
  // Failure switches take precedence over natural record state so switch behavior is deterministic.
  if (ctx.failure === 'data_conflict') {
    throw new R.AppError(409, 'data_conflict',
      `Purchase request ${pr.id} was already decisioned by another approver while you were reviewing it (simulated conflict).`,
      { requestId: pr.id, currentStatus: pr.status, simulated: true });
  }
  if (decision === 'approve' && ctx.failure === 'stale_entitlement') {
    throw new R.AppError(409, 'stale_entitlement',
      `Vendor contract for ${vendor ? vendor.company : pr.vendorId} is stale/expired (simulated): approval is blocked.`,
      { vendorId: pr.vendorId, simulated: true });
  }
  if (pr.status !== 'submitted' && pr.status !== 'screening') {
    throw new R.AppError(409, 'data_conflict',
      `Purchase request ${pr.id} is already in status "${pr.status}" — it cannot be decided again.`,
      { requestId: pr.id, currentStatus: pr.status });
  }
  if (decision === 'approve') {
    if (vendor && isPast(ctx, vendor.contractValidUntil)) {
      throw new R.AppError(409, 'stale_entitlement',
        `Vendor contract for ${vendor.company} expired on ${vendor.contractValidUntil} — a new vendor agreement must be in place before this request can be approved.`,
        { vendorId: pr.vendorId, contractValidUntil: vendor.contractValidUntil });
    }
    const po = {
      id: R.nextId(ctx.state, 'poSeq', 'po-'), requestId: pr.id, vendorId: pr.vendorId,
      totalUsd: pr.totalUsd, status: 'issued', issuedById: ctx.user.id, at: new Date().toISOString(),
    };
    pr.status = 'po_issued'; pr.poNumber = po.id;
    pr.approvals.push({ byId: ctx.user.id, decision: 'approve', note: String(b.note || ''), at: new Date().toISOString() });
    ctx.state.purchaseOrders.push(po);
    ctx.emit('procurement.approved', {
      subject: pr.id, summary: `PR ${pr.id} (${pr.item}) approved by ${ctx.user.name}`,
      data: { requestId: pr.id, totalUsd: pr.totalUsd, vendorId: pr.vendorId },
    });
    ctx.emit('po.issued', {
      subject: po.id, summary: `Purchase order ${po.id} issued to ${vendor ? vendor.company : pr.vendorId} for $${pr.totalUsd.toLocaleString('en-US')}`,
      data: { poId: po.id, requestId: pr.id, totalUsd: pr.totalUsd },
    });
    ctx.notify(pr.requestedById, 'procurement', `PR ${pr.id} approved — PO ${po.id} issued`,
      `${ctx.user.name} approved "${pr.item}". PO ${po.id} was issued to the vendor.`);
    const n2 = ctx.notify('u-5', 'procurement', `PO ${po.id} issued to your company`,
      `Purchase order ${po.id} (${pr.item}) was issued. Please confirm delivery schedule.`);
    return {
      message: `PR ${pr.id} approved; purchase order ${po.id} issued.`,
      warning: n2 && n2.status === 'failed' ? 'PO issued, but the vendor notification could not be delivered.' : null,
      redirect: '/procurement', purchaseRequest: pr, purchaseOrder: po,
    };
  }
  pr.status = 'rejected';
  pr.approvals.push({ byId: ctx.user.id, decision: 'reject', note: String(b.note || ''), at: new Date().toISOString() });
  ctx.emit('procurement.rejected', {
    subject: pr.id, summary: `PR ${pr.id} (${pr.item}) rejected by ${ctx.user.name}`,
    data: { requestId: pr.id, note: b.note || '' },
  });
  ctx.notify(pr.requestedById, 'procurement', `PR ${pr.id} rejected`, `${ctx.user.name} rejected "${pr.item}". Note: ${b.note || 'n/a'}`);
  return { message: `PR ${pr.id} rejected.`, redirect: '/procurement', purchaseRequest: pr };
}

function approveInvoice(ctx) {
  const b = ctx.body;
  const inv = find(ctx.state.invoices, b.invoiceId);
  if (!inv) throw new R.AppError(404, 'invoice_not_found', `Invoice "${b.invoiceId}" does not exist.`);
  if (ctx.failure === 'data_conflict') {
    throw new R.AppError(409, 'data_conflict', `Invoice ${inv.id} was already processed by another accountant (simulated conflict).`, { invoiceId: inv.id, currentStatus: inv.status, simulated: true });
  }
  if (inv.status !== 'received') {
    throw new R.AppError(409, 'data_conflict', `Invoice ${inv.id} is already "${inv.status}".`, { invoiceId: inv.id, currentStatus: inv.status });
  }
  inv.status = 'approved'; inv.approvedById = ctx.user.id; inv.at = new Date().toISOString();
  ctx.emit('invoice.approved', {
    subject: inv.id, summary: `Invoice ${inv.id} ($${inv.amountUsd.toLocaleString('en-US')}) approved for payment by ${ctx.user.name}`,
    data: { invoiceId: inv.id, amountUsd: inv.amountUsd, vendorId: inv.vendorId },
  });
  ctx.notify('u-1', 'finance', `Invoice ${inv.id} approved`, `${ctx.user.name} approved invoice ${inv.id} for payment.`);
  return { message: `Invoice ${inv.id} approved for payment.`, redirect: '/procurement', invoice: inv };
}

// ---------------------------------------------------------------- safety

function reportSafetyIncident(ctx) {
  const b = ctx.body;
  const project = find(ctx.state.projects, b.projectId);
  if (!project) throw new R.AppError(404, 'project_not_found', `Project "${b.projectId}" does not exist.`);
  const inc = {
    id: R.nextId(ctx.state, 'safetySeq', 'si-'), projectId: project.id,
    severity: ['minor', 'moderate', 'major'].includes(b.severity) ? b.severity : 'minor',
    category: String(b.category || 'general'), description: String(b.description || ''),
    status: 'open', reportedById: ctx.user.id, actions: [], reportedAt: new Date().toISOString(),
  };
  ctx.state.safetyIncidents.push(inc);
  ctx.emit('safety.incident_reported', {
    subject: inc.id, summary: `Safety incident (${inc.severity}/${inc.category}) reported on ${project.name}`,
    data: { incidentId: inc.id, projectId: project.id, severity: inc.severity, category: inc.category },
  });
  const n = ctx.notify('u-4', 'safety', `New ${inc.severity} incident on ${project.name}`, `${ctx.user.name} reported: ${inc.description}`);
  return {
    message: `Safety incident ${inc.id} recorded; the safety officer was notified.`,
    warning: n && n.status === 'failed' ? 'Incident saved, but the notification to the safety officer could not be delivered.' : null,
    redirect: '/safety', incident: inc,
  };
}

function closeSafetyIncident(ctx) {
  const b = ctx.body;
  const inc = find(ctx.state.safetyIncidents, b.incidentId);
  if (!inc) throw new R.AppError(404, 'incident_not_found', `Safety incident "${b.incidentId}" does not exist.`);
  if (ctx.failure === 'data_conflict') {
    throw new R.AppError(409, 'data_conflict', `Incident ${inc.id} was already closed by another officer (simulated conflict).`, { incidentId: inc.id, simulated: true });
  }
  if (inc.status === 'closed') throw new R.AppError(409, 'data_conflict', `Incident ${inc.id} is already closed.`, { incidentId: inc.id });
  inc.status = 'closed';
  inc.actions.push({ byId: ctx.user.id, text: String(b.resolution || 'Closed.'), at: new Date().toISOString() });
  ctx.emit('safety.incident_closed', {
    subject: inc.id, summary: `Safety incident ${inc.id} closed by ${ctx.user.name}`,
    data: { incidentId: inc.id, projectId: inc.projectId },
  });
  ctx.notify(inc.reportedById, 'safety', `Incident ${inc.id} closed`, `${ctx.user.name} closed incident ${inc.id}: ${b.resolution || ''}`);
  return { message: `Safety incident ${inc.id} closed.`, redirect: '/safety', incident: inc };
}

// ---------------------------------------------------------------- read helpers

function projectDetail(state, id) {
  const p = find(state.projects, id);
  if (!p) return null;
  return {
    project: p,
    tasks: state.tasks.filter((t) => t.projectId === id),
    reports: state.progressReports.filter((r) => r.projectId === id),
    purchaseRequests: state.purchaseRequests.filter((r) => r.projectId === id),
    incidents: state.safetyIncidents.filter((i) => i.projectId === id),
    documents: state.documents.filter((d) => d.projectId === id),
    photos: state.assets.filter((a) => a.projectId === id && a.kind === 'photo'),
    contractors: state.contractors.filter((c) => p.contractorIds.includes(c.id)),
  };
}

module.exports = {
  submitProgress, uploadPhoto, decidePurchaseRequest, approveInvoice,
  reportSafetyIncident, closeSafetyIncident, projectDetail, today, isPast,
};
