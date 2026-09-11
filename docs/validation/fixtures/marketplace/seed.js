'use strict';
/**
 * FlowMart seed world — a marketplace for "automation packages".
 *
 * IMPORTANT SCOPE NOTE (VWO-002 forbidden list): FlowMart models the STORE
 * lifecycle only — catalog, listings, versions, provenance, installs,
 * entitlements, upgrades, rollback, reviews. Packages are OPAQUE artifacts
 * (manifest + digest); this app does NOT define, interpret or execute
 * workflows. Workflow semantics belong to Codex Universal, not to fixtures.
 */

const crypto = require('node:crypto');

function user(id, username, name, role, permissions, tokenParts, passwordParts) {
  return { id, username, name, role, permissions, tokenParts, passwordParts };
}
function digestOf(manifest) {
  return 'sha256:' + crypto.createHash('sha256').update(JSON.stringify(manifest)).digest('hex').slice(0, 16);
}

function build() {
  return {
    users: [
      user('u-1', 'vera.osei', 'Vera Osei', 'publisher',
        ['listing:read', 'listing:publish', 'version:publish', 'entitlement:revoke', 'audit:verify'],
        ['fm', 'pub', 'vera', '13d7'], ['demo', 'pass', 'vera']),
      user('u-2', 'dan.kowalski', 'Dan Kowalski', 'marketplace_admin',
        ['listing:read', 'listing:publish', 'entitlement:grant', 'entitlement:revoke', 'audit:verify'],
        ['fm', 'adm', 'dan', '98c2'], ['demo', 'pass', 'dan']),
      user('u-3', 'iris.chen', 'Iris Chen', 'org_admin',
        ['listing:read', 'install:read', 'install:manage', 'install:configure', 'review:write'],
        ['fm', 'org', 'iris', '6e4b'], ['demo', 'pass', 'iris']),
      user('u-4', 'ravi.gupta', 'Ravi Gupta', 'org_member',
        ['listing:read', 'install:read', 'review:write'],
        ['fm', 'mem', 'ravi', 'a5f8'], ['demo', 'pass', 'ravi']),
      user('u-5', 'mia.torres', 'Mia Torres', 'auditor',
        ['listing:read', 'audit:verify'],
        ['fm', 'aud', 'mia', 'd2c1'], ['demo', 'pass', 'mia']),
      user('u-6', 'petra.voss', 'Petra Voss', 'org_admin',
        ['listing:read', 'install:read', 'install:manage', 'install:configure', 'review:write'],
        ['fm', 'org', 'petra', '70ab'], ['demo', 'pass', 'petra']),
    ],
    orgs: [
      { id: 'o-1', name: 'Northwind Logistics', adminId: 'u-3', memberIds: ['u-3', 'u-4'] },
      { id: 'o-2', name: 'Acme Retail', adminId: 'u-6', memberIds: ['u-6'] },
    ],
    packages: [
      {
        id: 'pk-101', slug: 'invoice-autofill', name: 'Invoice Autofill for ERPs', publisherId: 'u-1',
        category: 'finance-ops', licenseModel: 'subscription',
        summary: 'Extracts vendor invoice fields and fills ERP entry screens.',
        description: 'A package that reads inbound invoice documents, extracts structured fields and pre-fills the ERP invoice form. Ships with field mappings for three ERP products.',
        tags: ['invoice', 'erp', 'data-entry'],
        versions: [
          { version: '1.0.0', releasedAt: '2026-07-14', changelog: 'Initial release.',
            manifest: { name: 'invoice-autofill', version: '1.0.0', entry: 'package.json', files: ['manifest.json', 'mappings/erp-a.json', 'mappings/erp-b.json', 'mappings/erp-c.json'] },
            digest: digestOf({ name: 'invoice-autofill', version: '1.0.0', entry: 'package.json', files: ['manifest.json', 'mappings/erp-a.json', 'mappings/erp-b.json', 'mappings/erp-c.json'] }),
            provenance: { sourceRepo: 'https://git.example.internal/vera/invoice-autofill', commit: 'a1b2c3d', builtBy: 'vera.osei', signedBy: 'vera.osei' } },
          { version: '1.1.0', releasedAt: '2026-09-02', changelog: 'Adds ERP-C mapping; faster table detection.',
            manifest: { name: 'invoice-autofill', version: '1.1.0', entry: 'package.json', files: ['manifest.json', 'mappings/erp-a.json', 'mappings/erp-b.json', 'mappings/erp-c.json'] },
            digest: digestOf({ name: 'invoice-autofill', version: '1.1.0', entry: 'package.json', files: ['manifest.json', 'mappings/erp-a.json', 'mappings/erp-b.json', 'mappings/erp-c.json'] }),
            provenance: { sourceRepo: 'https://git.example.internal/vera/invoice-autofill', commit: 'e4f5a6b', builtBy: 'vera.osei', signedBy: 'vera.osei' } },
        ],
        stats: { installs: 412, stars: 4.6, reviews: 28 },
      },
      {
        id: 'pk-102', slug: 'support-triage-assistant', name: 'Support Ticket Triage Assistant', publisherId: 'u-1',
        category: 'support-ops', licenseModel: 'subscription',
        summary: 'Routes inbound support tickets to the right queue with confidence scores.',
        description: 'Classifies inbound tickets by topic, urgency and sentiment, then routes them to configured queues. Includes a review step for low-confidence classifications.',
        tags: ['support', 'routing', 'classification'],
        versions: [
          { version: '2.3.1', releasedAt: '2026-08-30', changelog: 'Fixes queue routing for merged tickets.',
            manifest: { name: 'support-triage-assistant', version: '2.3.1', entry: 'package.json', files: ['manifest.json', 'queues.json'] },
            digest: digestOf({ name: 'support-triage-assistant', version: '2.3.1', entry: 'package.json', files: ['manifest.json', 'queues.json'] }),
            provenance: { sourceRepo: 'https://git.example.internal/vera/support-triage-assistant', commit: 'c7d8e9f', builtBy: 'vera.osei', signedBy: 'vera.osei' } },
        ],
        stats: { installs: 1290, stars: 4.8, reviews: 96 },
      },
      {
        id: 'pk-103', slug: 'warehouse-slot-optimizer', name: 'Warehouse Slot Optimizer', publisherId: 'u-1',
        category: 'logistics', licenseModel: 'per-seat',
        summary: 'Suggests pick-face slotting for warehouse SKUs.',
        description: 'Reads SKU velocity and dimensions, then proposes slot assignments that cut average pick travel. Licensed per seat.',
        tags: ['warehouse', 'optimization'],
        versions: [
          { version: '1.2.0', releasedAt: '2026-09-08', changelog: 'Adds zone constraints.',
            manifest: { name: 'warehouse-slot-optimizer', version: '1.2.0', entry: 'package.json', files: ['manifest.json', 'zones.json'] },
            digest: digestOf({ name: 'warehouse-slot-optimizer', version: '1.2.0', entry: 'package.json', files: ['manifest.json', 'zones.json'] }),
            provenance: { sourceRepo: 'https://git.example.internal/vera/warehouse-slot-optimizer', commit: 'b3c4d5e', builtBy: 'vera.osei', signedBy: 'vera.osei' } },
        ],
        stats: { installs: 87, stars: 4.2, reviews: 11 },
      },
    ],
    entitlements: [
      { id: 'ent-0501', orgId: 'o-2', packageId: 'pk-102', type: 'subscription', seats: null, status: 'active', validUntil: '2027-05-01', grantedAt: '2026-08-30', grantedById: 'u-2' },
      { id: 'ent-0502', orgId: 'o-1', packageId: 'pk-101', type: 'trial', seats: null, status: 'expired', validUntil: '2026-08-01', grantedAt: '2026-07-15', grantedById: 'u-2' },
      { id: 'ent-0503', orgId: 'o-1', packageId: 'pk-103', type: 'per-seat', seats: 25, status: 'active', validUntil: '2027-01-01', grantedAt: '2026-09-01', grantedById: 'u-2' },
    ],
    installs: [
      { id: 'ins-0401', orgId: 'o-2', packageId: 'pk-102', version: '2.3.1', status: 'active',
        config: { queue: 'tier-1', confidenceThreshold: '0.7' }, installedById: 'u-6', installedAt: '2026-08-31T10:00:00Z',
        entitlementId: 'ent-0501', history: [{ action: 'installed', fromVersion: null, toVersion: '2.3.1', at: '2026-08-31T10:00:00Z', byId: 'u-6' }] },
    ],
    reviews: [
      { id: 'rv-0201', packageId: 'pk-102', orgId: 'o-1', byId: 'u-4', rating: 5, text: 'Cut our triage time in half.', at: '2026-09-05T12:00:00Z' },
    ],
    notifications: [
      { id: 'n-1', userId: 'u-3', to: 'iris.chen', type: 'marketplace', subject: 'Your trial of Invoice Autofill expired',
        text: 'The Northwind Logistics trial for pk-101 ended 2026-08-01. Renew to keep installing/upgrading.', status: 'delivered', createdAt: '2026-08-01T09:00:00Z' },
    ],
    events: [],
    meta: { today: '2026-09-12', pkSeq: 103, entSeq: 503, insSeq: 401, revSeq: 201 },
  };
}

module.exports = { build, digestOf };
