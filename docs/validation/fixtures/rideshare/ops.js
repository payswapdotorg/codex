'use strict';
/**
 * RidePilot domain operations — driver onboarding (public intake, screening,
 * activation) and support tickets. Failure switches take precedence over natural
 * record state.
 */

const R = require('../_lib/runtime');

function find(arr, id) { return arr.find((x) => x.id === id); }
function today(ctx) { return ctx.state.meta.today || new Date().toISOString().slice(0, 10); }
function isPast(ctx, d) { return String(d || '') < today(ctx); }
function plusDays(dateStr, days) {
  const d = new Date(dateStr + 'T00:00:00Z');
  d.setUTCDate(d.getUTCDate() + days);
  return d.toISOString().slice(0, 10);
}

function submitDriverApplication(ctx) {
  const b = ctx.body;
  const region = find(ctx.state.regions, b.regionId);
  if (!region) throw new R.AppError(404, 'region_not_found', `Region "${b.regionId}" does not exist.`);
  const plate = String(b.plate || '').trim().toUpperCase();
  if (!b.name || !plate) throw new R.AppError(400, 'validation_error', 'Name and vehicle plate are required.');
  if (ctx.state.driverApplications.find((a) => a.vehicle.plate === plate && ['submitted', 'screening', 'approved'].includes(a.status))) {
    throw new R.AppError(409, 'duplicate_event',
      `An active application for plate "${plate}" already exists — duplicate intake is rejected.`,
      { plate });
  }
  const app = {
    id: R.nextId(ctx.state, 'appSeq', 'da-'), name: String(b.name), phoneMasked: '***-***-' + String(b.phone || '0000').slice(-4),
    email: String(b.email || ''), regionId: region.id,
    vehicle: { make: String(b.make || '—'), model: String(b.model || '—'), year: parseInt(b.year, 10) || 2021, plate },
    docs: [
      { kind: 'license', status: b.licenseDoc === 'no' ? 'missing' : 'ok' },
      { kind: 'insurance', status: b.insuranceDoc === 'no' ? 'missing' : 'ok' },
      { kind: 'vehicle-inspection', status: 'ok' },
    ],
    backgroundCheck: 'pending', status: 'submitted', submittedAt: new Date().toISOString(),
    reviewedById: null, decisionNotes: '', preIssuedPermitValidUntil: plusDays(today(ctx), 365),
  };
  ctx.state.driverApplications.push(app);
  ctx.emit('driver.application_submitted', {
    subject: app.id, summary: `Driver application ${app.id} (${app.name}, ${region.code}) submitted via public intake`,
    data: { applicationId: app.id, regionId: region.id, plate },
  });
  const n = ctx.notify('u-2', 'onboarding', `New driver application: ${app.name}`,
    `Application ${app.id} (${region.name}) is ready for screening.`);
  return {
    message: `Application ${app.id} submitted for ${region.name}. Screening usually completes within 2 business days.`,
    warning: n && n.status === 'failed' ? 'Application recorded, but the ops notification could not be delivered.' : null,
    redirect: '/intake', application: app,
  };
}

function screenDriverApplication(ctx) {
  const b = ctx.body;
  const app = find(ctx.state.driverApplications, b.applicationId);
  if (!app) throw new R.AppError(404, 'application_not_found', `Driver application "${b.applicationId}" does not exist.`);
  if (ctx.failure === 'data_conflict') {
    throw new R.AppError(409, 'data_conflict', `Application ${app.id} was already decided by another ops reviewer (simulated).`, { applicationId: app.id, simulated: true });
  }
  if (app.status !== 'submitted' && app.status !== 'screening') {
    throw new R.AppError(409, 'data_conflict', `Application ${app.id} is already "${app.status}" — it cannot be screened again.`, { applicationId: app.id, status: app.status });
  }
  const decision = b.decision === 'reject' ? 'reject' : 'approve';
  if (decision === 'approve') {
    const missingDoc = app.docs.find((d) => d.status === 'missing');
    if (ctx.failure === 'missing_asset' || missingDoc) {
      throw new R.AppError(404, 'missing_asset',
        `Required document "${missingDoc ? missingDoc.kind : 'insurance'}" is not on file for application ${app.id}${missingDoc ? '' : ' (simulated dangling reference)'} — approval blocked.`,
        { applicationId: app.id, document: missingDoc ? missingDoc.kind : 'insurance', simulated: !missingDoc || undefined });
    }
    app.status = 'approved';
    app.reviewedById = ctx.user.id;
    app.decisionNotes = String(b.notes || '');
    app.backgroundCheck = 'passed';
    ctx.emit('driver.application_approved', {
      subject: app.id, summary: `Driver application ${app.id} (${app.name}) approved by ${ctx.user.name}`,
      data: { applicationId: app.id, regionId: app.regionId },
    });
    const n = ctx.notify('u-1', 'onboarding', `Application ${app.id} approved`,
      `${ctx.user.name} approved ${app.name}; activation is pending.`);
    return {
      message: `Application ${app.id} approved — ${app.name} can now be activated.`,
      warning: n && n.status === 'failed' ? 'Approval recorded, but the director notification could not be delivered.' : null,
      redirect: '/onboarding', application: app,
    };
  }
  app.status = 'rejected';
  app.reviewedById = ctx.user.id;
  app.decisionNotes = String(b.notes || '');
  ctx.emit('driver.application_rejected', {
    subject: app.id, summary: `Driver application ${app.id} (${app.name}) rejected by ${ctx.user.name}`,
    data: { applicationId: app.id, notes: app.decisionNotes },
  });
  return { message: `Application ${app.id} rejected.`, redirect: '/onboarding', application: app };
}

function activateDriver(ctx) {
  const b = ctx.body;
  const app = find(ctx.state.driverApplications, b.applicationId);
  if (!app) throw new R.AppError(404, 'application_not_found', `Driver application "${b.applicationId}" does not exist.`);
  if (ctx.failure === 'stale_entitlement') {
    throw new R.AppError(409, 'stale_entitlement',
      `Regional driver entitlement/permit for application ${app.id} is stale or expired (simulated): activation is blocked.`,
      { applicationId: app.id, simulated: true });
  }
  if (app.status !== 'approved') {
    throw new R.AppError(409, 'data_conflict', `Application ${app.id} is "${app.status}" — only approved applications can be activated.`, { applicationId: app.id, status: app.status });
  }
  if (isPast(ctx, app.preIssuedPermitValidUntil)) {
    throw new R.AppError(409, 'stale_entitlement',
      `Pre-issued permit for application ${app.id} expired on ${app.preIssuedPermitValidUntil} — re-screening is required before activation.`,
      { applicationId: app.id, permitValidUntil: app.preIssuedPermitValidUntil });
  }
  const driver = {
    id: R.nextId(ctx.state, 'driverSeq', 'dr-'), applicationId: app.id, name: app.name,
    regionId: app.regionId, status: 'offline', rating: 0, tripsCompleted: 0, earningsCents: 0,
    permitValidUntil: app.preIssuedPermitValidUntil, joinedAt: today(ctx),
  };
  ctx.state.drivers.push(driver);
  app.status = 'activated';
  const region = find(ctx.state.regions, app.regionId);
  region.activeDrivers += 1;
  const broadcast = {
    id: R.nextId(ctx.state, 'bSeq', 'b-'), regionId: region.id, channel: 'drivers',
    message: `Welcome to ${region.name}, ${driver.name}! First-trip guidance is available in the driver app.`,
    status: ctx.failure === 'notification_failure' ? 'failed' : 'sent',
    sentById: ctx.user.id, at: new Date().toISOString(),
  };
  ctx.state.broadcasts.push(broadcast);
  ctx.emit('driver.activated', {
    subject: driver.id, summary: `Driver ${driver.name} activated in ${region.name} by ${ctx.user.name} (permit until ${driver.permitValidUntil})`,
    data: { driverId: driver.id, applicationId: app.id, regionId: region.id },
  });
  if (broadcast.status === 'failed') {
    ctx.emit('notification.failed', {
      subject: `broadcast:${broadcast.id}`,
      summary: `Welcome broadcast to ${driver.name} FAILED to deliver (simulated switch)`,
      data: { broadcastId: broadcast.id, channel: 'drivers' },
    });
  }
  const n = ctx.notify('u-3', 'onboarding', `Driver ${driver.name} activated`, `${ctx.user.name} activated ${driver.name} in ${region.name}.`);
  return {
    message: `Driver ${driver.name} activated in ${region.name} (driver id ${driver.id}).`,
    warning: broadcast.status === 'failed' ? 'Driver activated, but the welcome broadcast failed to deliver (notification_failure).' : null,
    redirect: '/onboarding', driver, broadcast,
  };
}

// ---------------------------------------------------------------- support

function respondTicket(ctx) {
  const b = ctx.body;
  const t = find(ctx.state.supportTickets, b.ticketId);
  if (!t) throw new R.AppError(404, 'ticket_not_found', `Ticket "${b.ticketId}" does not exist.`);
  if (t.status === 'resolved') throw new R.AppError(409, 'data_conflict', `Ticket ${t.id} is already resolved.`, { ticketId: t.id });
  t.messages.push({ from: ctx.user.username, role: 'agent', at: new Date().toISOString(), text: String(b.message || '') });
  ctx.emit('support.message_added', {
    subject: t.id, summary: `${ctx.user.name} replied on ticket ${t.id} (${t.subject})`,
    data: { ticketId: t.id },
  });
  return { message: `Reply added to ticket ${t.id}.`, redirect: '/support', ticket: t };
}

function escalateTicket(ctx) {
  const b = ctx.body;
  const t = find(ctx.state.supportTickets, b.ticketId);
  if (!t) throw new R.AppError(404, 'ticket_not_found', `Ticket "${b.ticketId}" does not exist.`);
  if (ctx.failure === 'data_conflict') {
    throw new R.AppError(409, 'data_conflict', `Ticket ${t.id} was already escalated or resolved by another agent (simulated).`, { ticketId: t.id, simulated: true });
  }
  if (t.status !== 'open') {
    throw new R.AppError(409, 'data_conflict', `Ticket ${t.id} is "${t.status}" — only open tickets can be escalated.`, { ticketId: t.id, status: t.status });
  }
  t.status = 'escalated';
  t.messages.push({ from: ctx.user.username, role: 'agent', at: new Date().toISOString(), text: `Escalated: ${b.reason || 'needs ops attention'}` });
  ctx.emit('support.ticket_escalated', {
    subject: t.id, summary: `Ticket ${t.id} escalated by ${ctx.user.name} — ${b.reason || 'needs ops attention'}`,
    data: { ticketId: t.id, priority: t.priority },
  });
  const n = ctx.notify('u-2', 'support', `Ticket ${t.id} escalated: ${t.subject}`,
    `${ctx.user.name} escalated: ${b.reason || 'needs ops attention'}.`);
  return {
    message: `Ticket ${t.id} escalated to regional ops.`,
    warning: n && n.status === 'failed' ? 'Escalation recorded, but the ops notification could not be delivered.' : null,
    redirect: '/support', ticket: t,
  };
}

function resolveTicket(ctx) {
  const b = ctx.body;
  const t = find(ctx.state.supportTickets, b.ticketId);
  if (!t) throw new R.AppError(404, 'ticket_not_found', `Ticket "${b.ticketId}" does not exist.`);
  if (ctx.failure === 'data_conflict') {
    throw new R.AppError(409, 'data_conflict', `Ticket ${t.id} was resolved by another agent (simulated).`, { ticketId: t.id, simulated: true });
  }
  if (t.status === 'resolved') throw new R.AppError(409, 'data_conflict', `Ticket ${t.id} is already resolved.`, { ticketId: t.id });
  t.status = 'resolved';
  t.messages.push({ from: ctx.user.username, role: 'agent', at: new Date().toISOString(), text: `Resolved: ${b.resolution || 'handled'}` });
  ctx.emit('support.ticket_resolved', {
    subject: t.id, summary: `Ticket ${t.id} (${t.subject}) resolved by ${ctx.user.name}`,
    data: { ticketId: t.id },
  });
  return { message: `Ticket ${t.id} resolved.`, redirect: '/support', ticket: t };
}

module.exports = { submitDriverApplication, screenDriverApplication, activateDriver, respondTicket, escalateTicket, resolveTicket, today, isPast, plusDays, find };
