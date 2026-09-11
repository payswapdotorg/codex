'use strict';
/**
 * SiteBuild seed world — construction project operations.
 * All data is synthetic. Users carry token/password FRAGMENTS only; the runtime
 * assembles demo credentials from these fragments (no literal credentials in source).
 */

function user(id, username, name, role, permissions, tokenParts, passwordParts) {
  return { id, username, name, role, permissions, tokenParts, passwordParts };
}

function build() {
  return {
    users: [
      user('u-1', 'dana.reyes', 'Dana Reyes', 'project_manager',
        ['project:read', 'progress:read', 'po:approve', 'invoice:approve', 'doc:read', 'safety:read'],
        ['sb', 'pm', 'dana', '7c2f'], ['demo', 'pass', 'dana']),
      user('u-2', 'marco.silva', 'Marco Silva', 'site_foreman',
        ['project:read', 'progress:create', 'safety:report', 'doc:read'],
        ['sb', 'fore', 'marco', '91ae'], ['demo', 'pass', 'marco']),
      user('u-3', 'priya.nair', 'Priya Nair', 'finance_accountant',
        ['project:read', 'invoice:approve', 'po:read', 'doc:read'],
        ['sb', 'fin', 'priya', '5b81'], ['demo', 'pass', 'priya']),
      user('u-4', 'sam.oconnell', 'Sam O’Connell', 'safety_officer',
        ['project:read', 'safety:read', 'safety:close', 'doc:read', 'progress:read'],
        ['sb', 'saf', 'sam', 'e3d0'], ['demo', 'pass', 'sam']),
      user('u-5', 'lena.hart', 'Lena Hart', 'contractor_lead',
        ['project:read', 'progress:read', 'doc:read'],
        ['sb', 'con', 'lena', '44c9'], ['demo', 'pass', 'lena']),
    ],
    projects: [
      { id: 'p-101', code: 'HVT', name: 'Harborview Tower', site: '400 Pier Ave, Bay District', status: 'active',
        phase: 'structural', percentComplete: 62, budgetUsd: 4250000, spentUsd: 2641000,
        managerId: 'u-1', contractorIds: ['c-1', 'c-2'], dueDate: '2027-03-15' },
      { id: 'p-102', code: 'ESI', name: 'Eastside Interchange', site: 'Exit 14, Route 9', status: 'active',
        phase: 'earthwork', percentComplete: 35, budgetUsd: 8900000, spentUsd: 3115000,
        managerId: 'u-1', contractorIds: ['c-1'], dueDate: '2028-01-20' },
      { id: 'p-103', code: 'RVC', name: 'Riverside Clinic', site: '88 Millbrook Rd', status: 'active',
        phase: 'finishes', percentComplete: 88, budgetUsd: 1750000, spentUsd: 1542000,
        managerId: 'u-1', contractorIds: ['c-3'], dueDate: '2026-12-05' },
    ],
    tasks: [
      { id: 't-101a', projectId: 'p-101', title: 'Pour level-12 slab', trade: 'concrete', status: 'done', assigneeId: 'u-2' },
      { id: 't-101b', projectId: 'p-101', title: 'Erect tower crane B', trade: 'rigging', status: 'done', assigneeId: 'u-2' },
      { id: 't-101c', projectId: 'p-101', title: 'Level-13 formwork', trade: 'formwork', status: 'in_progress', assigneeId: 'u-2' },
      { id: 't-101d', projectId: 'p-101', title: 'Facade framing', trade: 'envelope', status: 'todo', assigneeId: null },
      { id: 't-102a', projectId: 'p-102', title: 'Utility relocation — north loop', trade: 'civil', status: 'in_progress', assigneeId: 'u-2' },
      { id: 't-102b', projectId: 'p-102', title: 'Retaining wall pour 2', trade: 'concrete', status: 'todo', assigneeId: null },
      { id: 't-103a', projectId: 'p-103', title: 'Punch list — level 2', trade: 'finishes', status: 'in_progress', assigneeId: 'u-2' },
      { id: 't-103b', projectId: 'p-103', title: 'Final coat — atrium', trade: 'finishes', status: 'todo', assigneeId: null },
    ],
    assets: [
      { id: 'a-101', projectId: 'p-101', kind: 'photo', name: 'site-overview-l12.jpg', sizeKb: 2400, caption: 'Level 12 slab pour — overview', checksum: 'b81f34c2', uploadedBy: 'u-2' },
      { id: 'a-102', projectId: 'p-101', kind: 'photo', name: 'crane-b-erection.jpg', sizeKb: 1850, caption: 'Tower crane B certified', checksum: '7caa9102', uploadedBy: 'u-2' },
      { id: 'a-103', projectId: 'p-102', kind: 'photo', name: 'eastside-excavation.jpg', sizeKb: 2100, caption: 'North loop utility trench', checksum: '30dd77a1', uploadedBy: 'u-2' },
      { id: 'a-104', projectId: 'p-103', kind: 'photo', name: 'riverside-facade.jpg', sizeKb: 1600, caption: 'Glazing complete — south facade', checksum: '99f1c0aa', uploadedBy: 'u-5' },
      { id: 'a-105', projectId: 'p-101', kind: 'document', name: 'structural-report-r3.pdf', sizeKb: 980, caption: 'Structural engineer report R3', checksum: '1f6b2290', uploadedBy: 'u-1' },
      { id: 'a-106', projectId: 'p-101', kind: 'document', name: 'site-safety-plan-v4.pdf', sizeKb: 640, caption: 'Site safety plan v4 (current)', checksum: 'aa72e5c4', uploadedBy: 'u-4' },
    ],
    progressReports: [
      { id: 'pr-4001', projectId: 'p-101', reportDate: '2026-09-10', authorId: 'u-2', taskIds: ['t-101a', 't-101b'],
        percentComplete: 60, notes: 'Slab pour complete; crane B certified and released for ops.', photoIds: ['a-101', 'a-102'] },
    ],
    purchaseRequests: [
      { id: 'prq-5001', projectId: 'p-101', item: 'High-tensile rebar Grade 60', qty: 1200, unit: 'tonne', unitCostUsd: 920,
        totalUsd: 1104000, vendorId: 'c-2', status: 'submitted', requestedById: 'u-2', approvals: [], poNumber: null, note: 'Level 13-15 structural package.' },
      { id: 'prq-5002', projectId: 'p-103', item: 'Low-e glazing panels', qty: 300, unit: 'panel', unitCostUsd: 480,
        totalUsd: 144000, vendorId: 'c-3', status: 'po_issued', requestedById: 'u-1',
        approvals: [{ byId: 'u-1', decision: 'approve', note: 'Within budget envelope.', at: '2026-09-02T10:12:00Z' }], poNumber: 'po-2041', note: 'South facade replacement stock.' },
    ],
    purchaseOrders: [
      { id: 'po-2041', requestId: 'prq-5002', vendorId: 'c-3', totalUsd: 144000, status: 'issued', issuedById: 'u-1', at: '2026-09-02T10:15:00Z' },
    ],
    invoices: [
      { id: 'inv-9001', vendorId: 'c-1', poNumber: 'po-2038', amountUsd: 182400, status: 'received', dueDate: '2026-10-01', note: 'Concrete deliveries Aug' },
      { id: 'inv-9002', vendorId: 'c-3', poNumber: 'po-2041', amountUsd: 96000, status: 'approved', approvedById: 'u-3', at: '2026-09-05T09:00:00Z', note: 'Glazing deposit' },
    ],
    safetyIncidents: [
      { id: 'si-2001', projectId: 'p-101', severity: 'minor', category: 'near-miss', description: 'Scaffold clamp dropped from level 8 barricaded zone; no injuries.',
        status: 'investigating', reportedById: 'u-2', actions: [{ byId: 'u-4', text: 'Barricade re-inspected; toolbox talk scheduled.', at: '2026-09-08T07:40:00Z' }] },
      { id: 'si-2002', projectId: 'p-102', severity: 'moderate', category: 'utility-strike', description: 'Telecom duct nicked during trenching; service restored same day.',
        status: 'closed', reportedById: 'u-2', actions: [{ byId: 'u-4', text: 'Closed after utility sign-off.', at: '2026-09-04T16:20:00Z' }] },
    ],
    documents: [
      { id: 'd-7001', projectId: 'p-101', name: 'Structural report R3', assetId: 'a-105', version: 3, owner: 'u-1' },
      { id: 'd-7002', projectId: 'p-101', name: 'Site safety plan v4', assetId: 'a-106', version: 4, owner: 'u-4' },
      { id: 'd-7003', projectId: 'p-102', name: 'Traffic control plan', assetId: null, version: 2, owner: 'u-1' },
    ],
    contractors: [
      { id: 'c-1', company: 'ConcreteWorks LLC', trade: 'concrete', contact: 'R. Alvarez', rating: 4.6, active: true, contractValidUntil: '2027-06-30' },
      { id: 'c-2', company: 'SteelCo Fabrication', trade: 'steel', contact: 'L. Hart', rating: 4.2, active: true, contractValidUntil: '2026-08-01' },
      { id: 'c-3', company: 'GlassWorks NW', trade: 'glazing', contact: 'M. Chen', rating: 4.8, active: true, contractValidUntil: '2027-01-15' },
    ],
    notifications: [
      { id: 'n-1', userId: 'u-1', to: 'dana.reyes', type: 'procurement', subject: 'PR prq-5001 awaiting approval',
        text: 'Purchase request prq-5001 (High-tensile rebar) needs your decision.', status: 'delivered', createdAt: '2026-09-11T08:00:00Z' },
    ],
    events: [],
    meta: { today: '2026-09-12', progressSeq: 4001, prqSeq: 5002, poSeq: 2041, invoiceSeq: 9002, safetySeq: 2002, assetSeq: 106, docSeq: 7003 },
  };
}

module.exports = { build };
