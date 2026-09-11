'use strict';
/**
 * ForgeOps seed world — software/Google-like enterprise (git forge, issues, CI,
 * deployments, incidents, services, docs, ownership).
 * Synthetic data only; credentials are runtime-assembled from fragments.
 */

function user(id, username, name, role, permissions, tokenParts, passwordParts) {
  return { id, username, name, role, permissions, tokenParts, passwordParts };
}

function build() {
  return {
    users: [
      user('u-1', 'ari.klein', 'Ari Klein', 'engineer',
        ['repo:read', 'pr:create', 'ci:run', 'issue:read', 'issue:create', 'issue:close', 'doc:read'],
        ['fo', 'eng', 'ari', '2f9c'], ['demo', 'pass', 'ari']),
      user('u-2', 'noor.haddad', 'Noor Haddad', 'code_owner',
        ['repo:read', 'pr:review', 'pr:merge', 'ci:run', 'issue:read', 'issue:create', 'issue:close', 'doc:read', 'doc:write', 'service:read'],
        ['fo', 'own', 'noor', '6ab1'], ['demo', 'pass', 'noor']),
      user('u-3', 'raj.patel', 'Raj Patel', 'release_manager',
        ['repo:read', 'ci:run', 'pr:merge', 'deploy:promote', 'deploy:rollback', 'service:read', 'doc:read', 'doc:write'],
        ['fo', 'rel', 'raj', 'd45e'], ['demo', 'pass', 'raj']),
      user('u-4', 'mei.chen', 'Mei Chen', 'sre_oncall',
        ['service:read', 'incident:open', 'incident:resolve', 'deploy:read', 'doc:read', 'doc:write', 'repo:read'],
        ['fo', 'sre', 'mei', '83f7'], ['demo', 'pass', 'mei']),
      user('u-5', 'tobi.oyelaran', 'Tobi Oyelaran', 'intern',
        ['repo:read', 'issue:create', 'doc:read'],
        ['fo', 'int', 'tobi', 'c019'], ['demo', 'pass', 'tobi']),
    ],
    repos: [
      { id: 'r-1', name: 'payments-service', language: 'Go', defaultBranch: 'main', ownerTeamId: 't-1', reviewerIds: ['u-2'], serviceName: 'payments-api', currentStagingVersion: '4.8.1', productionVersion: '4.8.1' },
      { id: 'r-2', name: 'web-checkout', language: 'TypeScript', defaultBranch: 'main', ownerTeamId: 't-2', reviewerIds: ['u-2'], serviceName: 'checkout-web', currentStagingVersion: '14.3.0', productionVersion: '14.3.0' },
      { id: 'r-3', name: 'search-indexer', language: 'Rust', defaultBranch: 'main', ownerTeamId: 't-1', reviewerIds: ['u-2'], serviceName: 'search-indexer', currentStagingVersion: '1.4.2', productionVersion: '1.4.1' },
    ],
    teams: [
      { id: 't-1', name: 'payments-core', mission: 'Payments and ledger services', memberIds: ['u-1', 'u-2', 'u-3'], ownsRepos: ['r-1', 'r-3'], ownsServices: ['s-1', 's-2'] },
      { id: 't-2', name: 'web-experience', mission: 'Storefront and checkout', memberIds: ['u-1', 'u-5'], ownsRepos: ['r-2'], ownsServices: ['s-3'] },
      { id: 't-3', name: 'platform-sre', mission: 'Reliability, deploys, on-call', memberIds: ['u-4', 'u-3'], ownsRepos: [], ownsServices: ['s-4'] },
    ],
    services: [
      { id: 's-1', name: 'payments-api', tier: 1, ownerTeamId: 't-1', status: 'healthy', uptime30d: 99.98, latencyMs: 84, version: '4.8.1', dependsOn: ['s-2'] },
      { id: 's-2', name: 'ledger-store', tier: 1, ownerTeamId: 't-1', status: 'healthy', uptime30d: 99.99, latencyMs: 12, version: '2.1.0', dependsOn: [] },
      { id: 's-3', name: 'checkout-web', tier: 2, ownerTeamId: 't-2', status: 'healthy', uptime30d: 99.91, latencyMs: 210, version: '14.3.0', dependsOn: ['s-1'] },
      { id: 's-4', name: 'search-indexer', tier: 2, ownerTeamId: 't-3', status: 'healthy', uptime30d: 99.87, latencyMs: 145, version: '1.4.1', dependsOn: ['s-2'] },
    ],
    pullRequests: [
      { id: 'pr-0301', repoId: 'r-1', number: 302, title: 'Refund idempotency keys', authorId: 'u-1', branch: 'ari/refund-idem', base: 'main',
        status: 'open', additions: 214, deletions: 38, approvals: [], ciRuns: ['run-0071'], summary: 'Adds idempotency keys to the refund path to stop duplicate refunds under retry.' },
      { id: 'pr-0302', repoId: 'r-2', number: 148, title: 'Checkout caching layer', authorId: 'u-5', branch: 'tobi/cache', base: 'main',
        status: 'open', additions: 482, deletions: 65, approvals: [], ciRuns: ['run-0072'], summary: 'Edge-caches cart pages for anonymous shoppers.' },
      { id: 'pr-0303', repoId: 'r-1', number: 298, title: 'Ledger reconcile fix', authorId: 'u-1', branch: 'ari/recon-fix', base: 'main',
        status: 'merged', additions: 96, deletions: 12, approvals: [{ byId: 'u-2', at: '2026-09-05T14:02:00Z', note: 'Solid fix.' }], ciRuns: ['run-0069'],
        mergedById: 'u-3', mergedAt: '2026-09-06T09:30:00Z', summary: 'Fixes off-by-one in nightly reconciliation.' },
    ],
    ciRuns: [
      { id: 'run-0071', prId: 'pr-0301', repoId: 'r-1', status: 'passed', startedAt: '2026-09-10T18:03:00Z',
        steps: [{ name: 'checkout', status: 'passed', durationSec: 8 }, { name: 'lint', status: 'passed', durationSec: 22 }, { name: 'unit-tests', status: 'passed', durationSec: 46 }, { name: 'integration-tests', status: 'passed', durationSec: 92 }] },
      { id: 'run-0072', prId: 'pr-0302', repoId: 'r-2', status: 'passed', startedAt: '2026-09-09T11:20:00Z',
        steps: [{ name: 'checkout', status: 'passed', durationSec: 7 }, { name: 'lint', status: 'passed', durationSec: 19 }, { name: 'unit-tests', status: 'passed', durationSec: 38 }, { name: 'e2e-smoke', status: 'passed', durationSec: 61 }] },
      { id: 'run-0069', prId: 'pr-0303', repoId: 'r-1', status: 'passed', startedAt: '2026-09-05T13:58:00Z',
        steps: [{ name: 'checkout', status: 'passed', durationSec: 8 }, { name: 'lint', status: 'passed', durationSec: 21 }, { name: 'unit-tests', status: 'passed', durationSec: 44 }] },
    ],
    deployments: [
      { id: 'dep-1101', repoId: 'r-1', prId: 'pr-0303', env: 'staging', version: '4.8.1', status: 'promoted', promotedById: 'u-3', artifactId: 'art-0901', at: '2026-09-06T09:35:00Z' },
      { id: 'dep-1102', repoId: 'r-1', prId: 'pr-0303', env: 'production', version: '4.8.1', status: 'succeeded', artifactId: 'art-0901', at: '2026-09-06T10:05:00Z' },
      { id: 'dep-1103', repoId: 'r-2', prId: 'pr-141', env: 'production', version: '14.3.0', status: 'succeeded', artifactId: 'art-0884', at: '2026-08-28T15:40:00Z' },
    ],
    artifacts: [
      { id: 'art-0901', repoId: 'r-1', digest: 'sha256:9f2c41ab77e0', sizeKb: 14320, builtFromPr: 'pr-0303' },
      { id: 'art-0884', repoId: 'r-2', digest: 'sha256:31ba90cc4e12', sizeKb: 8740, builtFromPr: 'pr-141' },
    ],
    incidents: [
      { id: 'inc-0901', severity: 'sev2', title: 'Duplicate refunds under retry', serviceId: 's-1', status: 'resolved',
        reporterId: 'u-4', assigneeId: 'u-4', openedAt: '2026-09-01T08:12:00Z', resolvedAt: '2026-09-01T13:45:00Z',
        description: 'Retried refund calls created duplicate ledger entries.',
        timeline: [{ at: '2026-09-01T08:12:00Z', byId: 'u-4', text: 'Opened after customer report.' }, { at: '2026-09-01T13:45:00Z', byId: 'u-4', text: 'Mitigated by disabling retry; fix tracked in pr-0301.' }] },
      { id: 'inc-0902', severity: 'sev3', title: 'Checkout latency spike at peak', serviceId: 's-3', status: 'mitigated',
        reporterId: 'u-4', assigneeId: 'u-4', openedAt: '2026-09-08T19:02:00Z', resolvedAt: null,
        description: 'p95 latency above 800ms during flash-sale traffic.',
        timeline: [{ at: '2026-09-08T19:02:00Z', byId: 'u-4', text: 'Opened from alert.' }, { at: '2026-09-08T19:40:00Z', byId: 'u-4', text: 'Scaled checkout-web; latency recovering. Root cause tracked in issue i-501.' }] },
    ],
    issues: [
      { id: 'i-0501', repoId: 'r-2', title: 'Checkout latency spike at peak', type: 'bug', priority: 'p1', status: 'open', assigneeId: 'u-4', labels: ['performance'] },
      { id: 'i-0502', repoId: 'r-1', title: 'Add idempotency keys to refunds', type: 'feature', priority: 'p2', status: 'open', assigneeId: 'u-1', labels: ['reliability'] },
      { id: 'i-0503', repoId: 'r-1', title: 'Flaky integration test: ledger reconcile', type: 'bug', priority: 'p3', status: 'in_progress', assigneeId: 'u-1', labels: ['flaky', 'ci'] },
    ],
    docs: [
      { id: 'doc-1', title: 'Runbook: payments deploys', path: '/docs/runbooks/payments-deploys', ownerTeamId: 't-3', version: 3, expectedVersion: 3,
        body: 'Deploy windows: Mon-Thu 10:00-16:00. Rollback: use the deployment console rollback action; ledger migrations are forward-only. Escalate to payments-core if reconcile drifts.' },
      { id: 'doc-2', title: 'On-call rotation & escalation', path: '/docs/oncall/rotation', ownerTeamId: 't-3', version: 2, expectedVersion: 2,
        body: 'Primary: platform-sre. Sev1 pages the director after 15 minutes. Sev2 after 30. Handover at 16:00.' },
      { id: 'doc-3', title: 'Service ownership directory', path: '/docs/ownership/services', ownerTeamId: 't-1', version: 1, expectedVersion: 1,
        body: 'payments-api & ledger-store: payments-core. checkout-web: web-experience. search-indexer: platform-sre.' },
    ],
    notifications: [
      { id: 'n-1', userId: 'u-2', to: 'noor.haddad', type: 'ci', subject: 'CI passed for #302 (refund idempotency keys)',
        text: 'PR pr-0301 passed all checks and is ready for review.', status: 'delivered', createdAt: '2026-09-10T18:07:00Z' },
    ],
    events: [],
    meta: { today: '2026-09-12', prSeq: 303, prNum: 302, runSeq: 72, depSeq: 1103, artSeq: 901, incSeq: 902, issueSeq: 503, docVersionSeq: {} },
  };
}

module.exports = { build };
