'use strict';
/**
 * ForgeOps domain operations — pull requests, CI, merge, issues.
 * Every mutation is exposed as a JSON API endpoint AND an HTML form (same handler).
 * Failure switches take precedence over natural record state (deterministic contract).
 */

const R = require('../_lib/runtime');

function find(arr, id) { return arr.find((x) => x.id === id); }
function findIssue(arr, id) { return arr.find((x) => x.id === id); }
function bumpPatch(v) {
  const parts = String(v).split('.');
  const n = parseInt(parts[2], 10) || 0;
  parts[2] = String(n + 1);
  return parts.join('.');
}

function runCIFor(ctx, pr, repo) {
  const run = {
    id: R.nextId(ctx.state, 'runSeq', 'run-'), prId: pr.id, repoId: repo.id,
    status: 'passed', startedAt: new Date().toISOString(),
    steps: [
      { name: 'checkout', status: 'passed', durationSec: 6 + (pr.additions % 5) },
      { name: 'lint', status: 'passed', durationSec: 18 + (pr.deletions % 7) },
      { name: 'unit-tests', status: 'passed', durationSec: 35 + (pr.additions % 30) },
      { name: 'integration-tests', status: 'passed', durationSec: 60 + (pr.additions % 40) },
    ],
  };
  ctx.state.ciRuns.push(run);
  pr.ciRuns.push(run.id);
  ctx.emit('ci.run_completed', {
    subject: run.id, summary: `CI ${run.status} for PR #${pr.number} (${pr.title}) on ${repo.name}`,
    data: { runId: run.id, prId: pr.id, repoId: repo.id, status: run.status },
  });
  return run;
}

function createPullRequest(ctx) {
  const b = ctx.body;
  const repo = find(ctx.state.repos, b.repoId);
  if (!repo) throw new R.AppError(404, 'repo_not_found', `Repository "${b.repoId}" does not exist.`);
  const branch = String(b.branch || '').trim();
  if (!branch) throw new R.AppError(400, 'validation_error', 'A source branch is required.');
  if (ctx.state.pullRequests.find((p) => p.repoId === repo.id && p.branch === branch && p.status === 'open')) {
    throw new R.AppError(409, 'duplicate_event',
      `An open PR from branch "${branch}" already exists on ${repo.name}.`,
      { repoId: repo.id, branch });
  }
  const pr = {
    id: R.nextId(ctx.state, 'prSeq', 'pr-'), repoId: repo.id,
    number: ctx.state.meta.prNum = (ctx.state.meta.prNum || 300) + 1,
    title: String(b.title || 'Untitled change'), authorId: ctx.user.id,
    branch, base: String(b.base || repo.defaultBranch), status: 'open',
    additions: parseInt(b.additions, 10) || 40, deletions: parseInt(b.deletions, 10) || 5,
    approvals: [], ciRuns: [], summary: String(b.summary || ''),
  };
  ctx.state.pullRequests.push(pr);
  ctx.emit('pr.opened', {
    subject: pr.id, summary: `PR #${pr.number} "${pr.title}" opened on ${repo.name} by ${ctx.user.name}`,
    data: { prId: pr.id, repoId: repo.id, number: pr.number, branch },
  });
  const run = runCIFor(ctx, pr, repo);
  const n = ctx.notify(repo.reviewerIds[0], 'ci', `PR #${pr.number} opened on ${repo.name}`,
    `${ctx.user.name} opened "${pr.title}". CI ${run.id} ${run.status}. Your review is requested.`);
  return {
    message: `PR #${pr.number} created; CI run ${run.id} passed.`,
    warning: n && n.status === 'failed' ? 'PR created, but the reviewer notification could not be delivered.' : null,
    redirect: `/prs/${pr.id}`, pullRequest: pr, ciRun: run,
  };
}

function rerunCI(ctx) {
  const b = ctx.body;
  const pr = find(ctx.state.pullRequests, b.prId);
  if (!pr) throw new R.AppError(404, 'pr_not_found', `Pull request "${b.prId}" does not exist.`);
  if (ctx.failure === 'data_conflict') {
    throw new R.AppError(409, 'data_conflict', `PR ${pr.id} moved while CI was being scheduled (simulated: base branch updated).`, { prId: pr.id, simulated: true });
  }
  if (pr.status !== 'open') {
    throw new R.AppError(409, 'data_conflict', `PR ${pr.id} is "${pr.status}" — CI can only be re-run on open PRs.`, { prId: pr.id, status: pr.status });
  }
  const repo = find(ctx.state.repos, pr.repoId);
  const run = runCIFor(ctx, pr, repo);
  return { message: `CI run ${run.id} passed for PR #${pr.number}.`, redirect: `/prs/${pr.id}`, ciRun: run };
}

function approvePR(ctx) {
  const b = ctx.body;
  const pr = find(ctx.state.pullRequests, b.prId);
  if (!pr) throw new R.AppError(404, 'pr_not_found', `Pull request "${b.prId}" does not exist.`);
  if (ctx.failure === 'data_conflict') {
    throw new R.AppError(409, 'data_conflict', `PR ${pr.id} changed state while approving (simulated).`, { prId: pr.id, simulated: true });
  }
  if (pr.authorId === ctx.user.id) {
    throw new R.AppError(403, 'permission_denied',
      `Authors cannot approve their own PR (self-approval blocked): ${ctx.user.username} authored PR #${pr.number}.`,
      { prId: pr.id, reason: 'self_approval', user: ctx.user.username });
  }
  if (pr.status !== 'open') {
    throw new R.AppError(409, 'data_conflict', `PR ${pr.id} is "${pr.status}" and can no longer be approved.`, { prId: pr.id, status: pr.status });
  }
  if (pr.approvals.find((a) => a.byId === ctx.user.id)) {
    throw new R.AppError(409, 'duplicate_event', `${ctx.user.name} already approved PR #${pr.number}.`, { prId: pr.id });
  }
  pr.approvals.push({ byId: ctx.user.id, at: new Date().toISOString(), note: String(b.note || '') });
  ctx.emit('pr.approved', {
    subject: pr.id, summary: `PR #${pr.number} "${pr.title}" approved by ${ctx.user.name}`,
    data: { prId: pr.id, approvals: pr.approvals.length },
  });
  const n = ctx.notify(pr.authorId, 'pr', `Your PR #${pr.number} was approved`, `${ctx.user.name} approved "${pr.title}". It is ready to merge.`);
  return {
    message: `Approval recorded for PR #${pr.number} (${pr.approvals.length} approval${pr.approvals.length === 1 ? '' : 's'}).`,
    warning: n && n.status === 'failed' ? 'Approval recorded, but the author notification could not be delivered.' : null,
    redirect: `/prs/${pr.id}`, pullRequest: pr,
  };
}

function mergePR(ctx) {
  const b = ctx.body;
  const pr = find(ctx.state.pullRequests, b.prId);
  if (!pr) throw new R.AppError(404, 'pr_not_found', `Pull request "${b.prId}" does not exist.`);
  const repo = find(ctx.state.repos, pr.repoId);
  if (ctx.failure === 'data_conflict') {
    throw new R.AppError(409, 'data_conflict',
      `Merge rejected (simulated): the base branch "${pr.base}" moved under the merge window; rebase required.`,
      { prId: pr.id, base: pr.base, simulated: true });
  }
  if (pr.status !== 'open') {
    throw new R.AppError(409, 'data_conflict', `PR ${pr.id} is "${pr.status}" — only open PRs can be merged.`, { prId: pr.id, status: pr.status });
  }
  if (!pr.approvals.length) {
    throw new R.AppError(400, 'pr_not_approved', `PR #${pr.number} has no approvals — at least one reviewer approval is required before merge.`, { prId: pr.id });
  }
  const lastRun = pr.ciRuns.length ? find(ctx.state.ciRuns, pr.ciRuns[pr.ciRuns.length - 1]) : null;
  if (!lastRun || lastRun.status !== 'passed') {
    throw new R.AppError(400, 'ci_not_green', `PR #${pr.number} has no passing CI run — merge is blocked.`, { prId: pr.id });
  }
  const version = bumpPatch(repo.currentStagingVersion);
  repo.currentStagingVersion = version;
  const artifact = {
    id: R.nextId(ctx.state, 'artSeq', 'art-'), repoId: repo.id,
    digest: 'sha256:' + require('node:crypto').createHash('sha256').update(`${repo.id}:${version}:${pr.id}`).digest('hex').slice(0, 12),
    sizeKb: 9000 + (pr.additions * 7) % 9000, builtFromPr: pr.id,
  };
  ctx.state.artifacts.push(artifact);
  const dep = {
    id: R.nextId(ctx.state, 'depSeq', 'dep-'), repoId: repo.id, prId: pr.id,
    env: 'staging', version, status: 'succeeded', artifactId: artifact.id, at: new Date().toISOString(),
  };
  ctx.state.deployments.push(dep);
  pr.status = 'merged'; pr.mergedById = ctx.user.id; pr.mergedAt = new Date().toISOString();
  ctx.emit('pr.merged', {
    subject: pr.id, summary: `PR #${pr.number} "${pr.title}" merged by ${ctx.user.name}; ${repo.name} staging → ${version}`,
    data: { prId: pr.id, repoId: repo.id, version },
  });
  ctx.emit('deployment.created', {
    subject: dep.id, summary: `Staging deployment ${dep.id} of ${repo.name} ${version} (artifact ${artifact.id})`,
    data: { deploymentId: dep.id, repoId: repo.id, env: 'staging', version, artifactId: artifact.id },
  });
  const n = ctx.notify('u-3', 'deploy', `${repo.name} ${version} staged — promotion approval needed`,
    `PR #${pr.number} merged. Staging deployment ${dep.id} is awaiting your promotion decision.`);
  return {
    message: `PR #${pr.number} merged; ${repo.name} ${version} deployed to staging (deployment ${dep.id}).`,
    warning: n && n.status === 'failed' ? 'Merge completed, but the release-manager notification could not be delivered.' : null,
    redirect: `/repos/${repo.id}`, pullRequest: pr, deployment: dep, artifact,
  };
}

function createIssue(ctx) {
  const b = ctx.body;
  const repo = find(ctx.state.repos, b.repoId);
  if (!repo) throw new R.AppError(404, 'repo_not_found', `Repository "${b.repoId}" does not exist.`);
  const title = String(b.title || '').trim();
  if (!title) throw new R.AppError(400, 'validation_error', 'Issue title is required.');
  if (ctx.state.issues.find((i) => i.repoId === repo.id && i.title === title && i.status !== 'closed')) {
    throw new R.AppError(409, 'duplicate_event', `An open issue titled "${title}" already exists on ${repo.name}.`, { repoId: repo.id });
  }
  const issue = {
    id: R.nextId(ctx.state, 'issueSeq', 'i-'), repoId: repo.id, title,
    type: ['bug', 'feature', 'task'].includes(b.type) ? b.type : 'task',
    priority: ['p0', 'p1', 'p2', 'p3'].includes(b.priority) ? b.priority : 'p2',
    status: 'open', assigneeId: b.assigneeId && R.findUserById(ctx.state, b.assigneeId) ? b.assigneeId : null,
    labels: R.asList(b.labels),
  };
  ctx.state.issues.push(issue);
  ctx.emit('issue.created', {
    subject: issue.id, summary: `Issue ${issue.id} "${issue.title}" filed on ${repo.name} by ${ctx.user.name}`,
    data: { issueId: issue.id, repoId: repo.id, type: issue.type, priority: issue.priority },
  });
  return { message: `Issue ${issue.id} created.`, redirect: '/issues', issue };
}

function closeIssue(ctx) {
  const b = ctx.body;
  const issue = findIssue(ctx.state.issues, b.issueId);
  if (!issue) throw new R.AppError(404, 'issue_not_found', `Issue "${b.issueId}" does not exist.`);
  if (ctx.failure === 'data_conflict') {
    throw new R.AppError(409, 'data_conflict', `Issue ${issue.id} was closed by someone else while you were closing it (simulated).`, { issueId: issue.id, simulated: true });
  }
  if (issue.status === 'closed') {
    throw new R.AppError(409, 'data_conflict', `Issue ${issue.id} is already closed.`, { issueId: issue.id });
  }
  issue.status = 'closed';
  ctx.emit('issue.closed', {
    subject: issue.id, summary: `Issue ${issue.id} "${issue.title}" closed by ${ctx.user.name}`,
    data: { issueId: issue.id, repoId: issue.repoId },
  });
  return { message: `Issue ${issue.id} closed.`, redirect: '/issues', issue };
}

module.exports = { createPullRequest, rerunCI, approvePR, mergePR, createIssue, closeIssue, runCIFor, bumpPatch, find };
