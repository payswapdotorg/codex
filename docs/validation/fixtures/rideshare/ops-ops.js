'use strict';
/**
 * RidePilot domain operations — surge, broadcasts, payouts, incidents.
 */

const R = require('../_lib/runtime');
const D = require('./ops');
const { find, today, isPast } = D;

function adjustSurge(ctx) {
  const b = ctx.body;
  const region = find(ctx.state.regions, b.regionId);
  if (!region) throw new R.AppError(404, 'region_not_found', `Region "${b.regionId}" does not exist.`);
  if (ctx.failure === 'data_conflict') {
    throw new R.AppError(409, 'data_conflict', `Another ops manager adjusted ${region.name} surge while you were saving (simulated).`, { regionId: region.id, simulated: true });
  }
  const level = Math.round(parseFloat(b.level) * 10) / 10;
  if (Number.isNaN(level) || level < 1.0 || level > 3.0) {
    throw new R.AppError(400, 'validation_error', 'Surge level must be between 1.0 and 3.0.');
  }
  const before = region.surgeLevel;
  region.surgeLevel = level;
  region.status = level >= 1.5 ? 'surge' : 'normal';
  const broadcast = {
    id: R.nextId(ctx.state, 'bSeq', 'b-'), regionId: region.id, channel: 'drivers',
    message: `Surge in ${region.name} is now ${level.toFixed(1)}× (was ${before.toFixed(1)}×).`,
    status: ctx.failure === 'notification_failure' ? 'failed' : 'sent',
    sentById: ctx.user.id, at: new Date().toISOString(),
  };
  ctx.state.broadcasts.push(broadcast);
  ctx.emit('ops.surge_adjusted', {
    subject: region.id, summary: `${region.name} surge ${before.toFixed(1)}× → ${level.toFixed(1)}× by ${ctx.user.name}`,
    data: { regionId: region.id, from: before, to: level, broadcastId: broadcast.id },
  });
  if (broadcast.status === 'failed') {
    ctx.emit('notification.failed', {
      subject: `broadcast:${broadcast.id}`,
      summary: `Surge broadcast to ${region.name} drivers FAILED to deliver (simulated switch)`,
      data: { broadcastId: broadcast.id, regionId: region.id },
    });
  }
  return {
    message: `${region.name} surge set to ${level.toFixed(1)}×; ${broadcast.status === 'sent' ? 'drivers notified' : 'driver broadcast FAILED'}.`,
    warning: broadcast.status === 'failed' ? 'Surge updated, but the driver broadcast could not be delivered (notification_failure).' : null,
    redirect: '/ops', region, broadcast,
  };
}

function approvePayout(ctx) {
  const b = ctx.body;
  const pay = find(ctx.state.payments, b.paymentId);
  if (!pay) throw new R.AppError(404, 'payment_not_found', `Payment "${b.paymentId}" does not exist.`);
  const driver = find(ctx.state.drivers, pay.driverId);
  if (ctx.failure === 'data_conflict') {
    throw new R.AppError(409, 'data_conflict', `Payout ${pay.id} was already processed by another analyst (simulated).`, { paymentId: pay.id, simulated: true });
  }
  if (pay.status !== 'pending') {
    throw new R.AppError(409, 'data_conflict', `Payout ${pay.id} is already "${pay.status}".`, { paymentId: pay.id, status: pay.status });
  }
  if (ctx.failure === 'stale_entitlement' || (driver && isPast(ctx, driver.permitValidUntil))) {
    throw new R.AppError(409, 'stale_entitlement',
      `Driver ${driver ? driver.name : pay.driverId}'s regional permit expired on ${driver ? driver.permitValidUntil : '?'} — payouts require a valid permit.`,
      { paymentId: pay.id, driverId: pay.driverId, permitValidUntil: driver && driver.permitValidUntil, simulated: ctx.failure === 'stale_entitlement' || undefined });
  }
  pay.status = 'approved'; pay.approvedById = ctx.user.id; pay.approvedAt = new Date().toISOString();
  ctx.emit('payout.approved', {
    subject: pay.id, summary: `Payout ${pay.id} to ${driver ? driver.name : pay.driverId} ($${(pay.netCents / 100).toFixed(2)}) approved by ${ctx.user.name}`,
    data: { paymentId: pay.id, driverId: pay.driverId, netCents: pay.netCents },
  });
  return { message: `Payout ${pay.id} approved for payment.`, redirect: '/payments', payment: pay };
}

function reportIncident(ctx) {
  const b = ctx.body;
  const driver = b.driverId ? find(ctx.state.drivers, b.driverId) : null;
  if (b.driverId && !driver) throw new R.AppError(404, 'driver_not_found', `Driver "${b.driverId}" does not exist.`);
  const inc = {
    id: R.nextId(ctx.state, 'incSeq', 'in-'), tripId: b.tripId || null, driverId: driver ? driver.id : null,
    category: String(b.category || 'other'), severity: ['low', 'medium', 'high'].includes(b.severity) ? b.severity : 'low',
    status: 'investigating', description: String(b.description || ''),
    reportedAt: new Date().toISOString(), reportedById: ctx.user.id, resolution: '',
  };
  ctx.state.incidents.push(inc);
  ctx.emit('incident.reported', {
    subject: inc.id, summary: `${inc.severity} incident (${inc.category}) reported${driver ? ' on driver ' + driver.name : ''}`,
    data: { incidentId: inc.id, driverId: inc.driverId, tripId: inc.tripId },
  });
  const n = ctx.notify('u-1', 'incident', `New ${inc.severity} incident ${inc.id}`, `${ctx.user.name} reported: ${inc.description}`);
  return {
    message: `Incident ${inc.id} recorded and routed to operations.`,
    warning: n && n.status === 'failed' ? 'Incident saved, but the ops-director notification could not be delivered.' : null,
    redirect: '/incidents', incident: inc,
  };
}

function resolveIncident(ctx) {
  const b = ctx.body;
  const inc = find(ctx.state.incidents, b.incidentId);
  if (!inc) throw new R.AppError(404, 'incident_not_found', `Incident "${b.incidentId}" does not exist.`);
  if (ctx.failure === 'data_conflict') {
    throw new R.AppError(409, 'data_conflict', `Incident ${inc.id} was resolved by another ops manager (simulated).`, { incidentId: inc.id, simulated: true });
  }
  if (inc.status === 'resolved') throw new R.AppError(409, 'data_conflict', `Incident ${inc.id} is already resolved.`, { incidentId: inc.id });
  inc.status = 'resolved';
  inc.resolution = String(b.resolution || 'Resolved.');
  ctx.emit('incident.resolved', {
    subject: inc.id, summary: `Incident ${inc.id} resolved by ${ctx.user.name}: ${inc.resolution}`,
    data: { incidentId: inc.id },
  });
  const n = ctx.notify(inc.reportedById, 'incident', `Incident ${inc.id} resolved`, inc.resolution);
  return {
    message: `Incident ${inc.id} resolved.`,
    warning: n && n.status === 'failed' ? 'Incident resolved, but the reporter notification could not be delivered.' : null,
    redirect: '/incidents', incident: inc,
  };
}

module.exports = { adjustSurge, approvePayout, reportIncident, resolveIncident };
