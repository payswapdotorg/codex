'use strict';
/**
 * RidePilot seed world — ride-share operations (onboarding, support, ops, trips,
 * regions, maps, payments, incidents, communications). Synthetic data only.
 */

function user(id, username, name, role, permissions, tokenParts, passwordParts) {
  return { id, username, name, role, permissions, tokenParts, passwordParts };
}

function build() {
  return {
    users: [
      user('u-1', 'farah.khan', 'Farah Khan', 'ops_director',
        ['driver:read', 'driver:screen', 'driver:activate', 'surge:adjust', 'broadcast:send', 'payout:approve', 'incident:report', 'incident:resolve', 'ticket:read', 'ticket:resolve'],
        ['rp', 'dir', 'farah', '77af'], ['demo', 'pass', 'farah']),
      user('u-2', 'jules.moreau', 'Jules Moreau', 'regional_ops_manager',
        ['driver:read', 'driver:screen', 'driver:activate', 'surge:adjust', 'broadcast:send', 'incident:report', 'incident:resolve', 'ticket:read', 'ticket:resolve'],
        ['rp', 'ops', 'jules', 'b318'], ['demo', 'pass', 'jules']),
      user('u-3', 'ana.silva', 'Ana Silva', 'support_agent',
        ['driver:read', 'ticket:read', 'ticket:respond', 'ticket:escalate', 'incident:report'],
        ['rp', 'sup', 'ana', '5d92'], ['demo', 'pass', 'ana']),
      user('u-4', 'kwame.mensah', 'Kwame Mensah', 'finance_analyst',
        ['driver:read', 'payout:approve'],
        ['rp', 'fin', 'kwame', 'e4c6'], ['demo', 'pass', 'kwame']),
    ],
    regions: [
      { id: 'rg-1', name: 'Downtown Core', code: 'DT', surgeLevel: 1.4, activeDrivers: 34, demandIndex: 82, status: 'normal', map: { x: 1, y: 1, w: 2, h: 2 } },
      { id: 'rg-2', name: 'Airport Corridor', code: 'AP', surgeLevel: 1.8, activeDrivers: 12, demandIndex: 71, status: 'surge', map: { x: 4, y: 0, w: 2, h: 2 } },
      { id: 'rg-3', name: 'Northside', code: 'NS', surgeLevel: 1.0, activeDrivers: 9, demandIndex: 24, status: 'normal', map: { x: 0, y: 0, w: 1, h: 2 } },
      { id: 'rg-4', name: 'Riverside', code: 'RV', surgeLevel: 1.2, activeDrivers: 15, demandIndex: 38, status: 'normal', map: { x: 2, y: 3, w: 2, h: 1 } },
    ],
    driverApplications: [
      { id: 'da-0301', name: 'Marcus Webb', phoneMasked: '***-***-4481', email: 'marcus.webb@example.test', regionId: 'rg-1',
        vehicle: { make: 'Honda', model: 'Civic', year: 2022, plate: 'WEB-2481' },
        docs: [{ kind: 'license', status: 'ok' }, { kind: 'insurance', status: 'ok' }, { kind: 'vehicle-inspection', status: 'ok' }],
        backgroundCheck: 'passed', status: 'submitted', submittedAt: '2026-09-10T09:15:00Z', reviewedById: null, decisionNotes: '', preIssuedPermitValidUntil: '2027-09-01' },
      { id: 'da-0302', name: 'Elena Petrova', phoneMasked: '***-***-7712', email: 'elena.petrova@example.test', regionId: 'rg-2',
        vehicle: { make: 'Toyota', model: 'Camry', year: 2021, plate: 'PET-7702' },
        docs: [{ kind: 'license', status: 'ok' }, { kind: 'insurance', status: 'missing' }, { kind: 'vehicle-inspection', status: 'ok' }],
        backgroundCheck: 'pending', status: 'screening', submittedAt: '2026-09-08T14:40:00Z', reviewedById: null, decisionNotes: '', preIssuedPermitValidUntil: '2027-09-01' },
      { id: 'da-0303', name: 'Tomás Rivera', phoneMasked: '***-***-2210', email: 'tomas.rivera@example.test', regionId: 'rg-3',
        vehicle: { make: 'Ford', model: 'Focus', year: 2020, plate: 'RIV-2210' },
        docs: [{ kind: 'license', status: 'ok' }, { kind: 'insurance', status: 'ok' }, { kind: 'vehicle-inspection', status: 'ok' }],
        backgroundCheck: 'passed', status: 'approved', submittedAt: '2026-08-20T10:00:00Z',
        reviewedById: 'u-2', decisionNotes: 'All checks green; activation pending.', preIssuedPermitValidUntil: '2026-08-01' },
    ],
    drivers: [
      { id: 'dr-0501', applicationId: null, name: 'Aisha Nasser', regionId: 'rg-1', status: 'online', rating: 4.92, tripsCompleted: 1284, earningsCents: 1824050, permitValidUntil: '2027-03-01', joinedAt: '2025-11-02' },
      { id: 'dr-0502', applicationId: null, name: 'Ben Okafor', regionId: 'rg-4', status: 'offline', rating: 4.71, tripsCompleted: 642, earningsCents: 902100, permitValidUntil: '2026-10-15', joinedAt: '2026-01-18' },
      { id: 'dr-0503', applicationId: null, name: 'Lena Marsh', regionId: 'rg-1', status: 'online', rating: 4.85, tripsCompleted: 901, earningsCents: 1288700, permitValidUntil: '2026-07-01', joinedAt: '2026-02-05' },
    ],
    supportTickets: [
      { id: 'tk-0701', openedBy: 'passenger (Nadia F.)', subject: 'Charged twice for a cancelled trip', category: 'billing', priority: 'high', status: 'open', assigneeId: 'u-3',
        messages: [{ from: 'passenger', role: 'customer', at: '2026-09-11T16:20:00Z', text: 'My card was charged twice after I cancelled within the free window.' },
                  { from: 'ana.silva', role: 'agent', at: '2026-09-11T16:41:00Z', text: 'Thanks — I can see the duplicate authorization. Requesting a reversal.' }] },
      { id: 'tk-0702', openedBy: 'driver (Ilya V.)', subject: 'App crashes when ending a trip', category: 'technical', priority: 'medium', status: 'escalated', assigneeId: 'u-3',
        messages: [{ from: 'driver', role: 'customer', at: '2026-09-09T08:05:00Z', text: 'The app force-closes on trip end; fare shows 0 afterwards.' },
                  { from: 'ana.silva', role: 'agent', at: '2026-09-09T08:30:00Z', text: 'Reproduced on Android 14. Escalating to engineering.' }] },
    ],
    trips: [
      { id: 'tr-0801', regionId: 'rg-1', driverId: 'dr-0501', passenger: 'R. Alvarez', status: 'completed', fareCents: 1840, distanceKm: 6.8, minutes: 19, requestedAt: '2026-09-11T17:32:00Z' },
      { id: 'tr-0802', regionId: 'rg-2', driverId: 'dr-0501', passenger: 'S. Whitfield', status: 'completed', fareCents: 3120, distanceKm: 22.4, minutes: 34, requestedAt: '2026-09-11T14:08:00Z' },
      { id: 'tr-0803', regionId: 'rg-4', driverId: 'dr-0502', passenger: 'M. Duarte', status: 'completed', fareCents: 990, distanceKm: 3.1, minutes: 11, requestedAt: '2026-09-10T21:47:00Z' },
      { id: 'tr-0804', regionId: 'rg-2', driverId: null, passenger: 'K. Nowak', status: 'requested', fareCents: 0, distanceKm: 18.2, minutes: 0, requestedAt: '2026-09-12T07:55:00Z' },
    ],
    payments: [
      { id: 'pay-0901', driverId: 'dr-0501', payPeriod: '2026-08-25 → 2026-08-31', tripsCount: 62, grossCents: 81240, commissionCents: 16248, netCents: 64992, status: 'pending' },
      { id: 'pay-0902', driverId: 'dr-0502', payPeriod: '2026-08-25 → 2026-08-31', tripsCount: 41, grossCents: 52980, commissionCents: 10596, netCents: 42384, status: 'approved', approvedById: 'u-4' },
      { id: 'pay-0903', driverId: 'dr-0503', payPeriod: '2026-08-25 → 2026-08-31', tripsCount: 55, grossCents: 73110, commissionCents: 14622, netCents: 58488, status: 'pending' },
    ],
    incidents: [
      { id: 'in-0601', tripId: 'tr-0802', driverId: 'dr-0501', category: 'unsafe-driving', severity: 'low', status: 'investigating',
        description: 'Passenger reported an unsafe lane change near the airport merge.', reportedAt: '2026-09-11T15:10:00Z', reportedById: 'u-3', resolution: '' },
    ],
    broadcasts: [
      { id: 'b-0401', regionId: 'rg-1', channel: 'drivers', message: 'High demand expected 17:00–19:00 in Downtown Core — position near transit hubs.', status: 'sent', sentById: 'u-2', at: '2026-09-11T15:45:00Z' },
    ],
    notifications: [
      { id: 'n-1', userId: 'u-2', to: 'jules.moreau', type: 'onboarding', subject: 'New driver application: Marcus Webb',
        text: 'Application da-0301 (Downtown Core) is ready for screening.', status: 'delivered', createdAt: '2026-09-10T09:16:00Z' },
    ],
    events: [],
    meta: { today: '2026-09-12', appSeq: 303, driverSeq: 503, ticketSeq: 702, tripSeq: 804, paySeq: 903, incSeq: 601, bSeq: 401 },
  };
}

module.exports = { build };
