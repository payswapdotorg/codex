'use strict';
/**
 * PressRoom domain operations — editorial story lifecycle (draft → review →
 * approve → publish → correct) with optimistic versioning. Failure switches
 * take precedence over natural record state.
 */

const R = require('../_lib/runtime');

function find(arr, id) { return arr.find((x) => x.id === id); }
function today(ctx) { return ctx.state.meta.today || new Date().toISOString().slice(0, 10); }
function isPast(ctx, d) { return String(d || '9999-12-31') < today(ctx); }

function checkVersion(ctx, story, field) {
  const expected = parseInt(ctx.body.expectedVersion, 10);
  if (!Number.isNaN(expected) && expected !== story.version) {
    throw new R.AppError(409, 'data_conflict',
      `Version mismatch on story ${story.id}: you worked from v${expected} but the current version is v${story.version} — reload and reapply.`,
      { storyId: story.id, currentVersion: story.version, yourVersion: expected });
  }
}

function createStory(ctx) {
  const b = ctx.body;
  const title = String(b.title || '').trim();
  if (!title) throw new R.AppError(400, 'validation_error', 'A story title is required.');
  if (ctx.state.stories.find((s) => s.title === title && s.status !== 'archived')) {
    throw new R.AppError(409, 'duplicate_event', `A story titled "${title}" already exists in the CMS.`, { title });
  }
  const body = String(b.body || '');
  const story = {
    id: R.nextId(ctx.state, 'storySeq', 'st-'), slug: title.toLowerCase().replace(/[^a-z0-9]+/g, '-').replace(/^-|-$/g, '').slice(0, 60),
    title, section: ctx.state.sections.includes(b.section) ? b.section : 'news', authorId: ctx.user.id,
    dek: String(b.dek || ''), body, status: 'draft', version: 1, expectedVersion: 1,
    wordCount: body.split(/\s+/).filter(Boolean).length,
    assets: [], channels: [], submittedAt: null, publishedAt: null, approvedById: null, socialPosts: [],
  };
  ctx.state.stories.push(story);
  ctx.emit('story.created', {
    subject: story.id, summary: `Story "${story.title}" created as draft by ${ctx.user.name}`,
    data: { storyId: story.id, section: story.section },
  });
  return { message: `Story ${story.id} ("${story.title}") created as a draft.`, redirect: `/stories/${story.id}`, story };
}

function editStory(ctx) {
  const b = ctx.body;
  const story = find(ctx.state.stories, b.storyId);
  if (!story) throw new R.AppError(404, 'story_not_found', `Story "${b.storyId}" does not exist.`);
  const isEditor = ctx.user.permissions.includes('story:approve');
  if (story.authorId !== ctx.user.id && !isEditor) {
    throw new R.AppError(403, 'permission_denied',
      `Only the author or an editor can edit story ${story.id} (you are ${ctx.user.username}).`,
      { storyId: story.id, required: 'story:edit (own or editor)' });
  }
  if (ctx.failure === 'data_conflict') {
    throw new R.AppError(409, 'data_conflict', `Story ${story.id} was edited by someone else while you were saving (simulated).`, { storyId: story.id, simulated: true });
  }
  checkVersion(ctx, story);
  if (b.title !== undefined && String(b.title).trim()) story.title = String(b.title).trim();
  if (b.dek !== undefined) story.dek = String(b.dek);
  if (b.body !== undefined) {
    story.body = String(b.body);
    story.wordCount = story.body.split(/\s+/).filter(Boolean).length;
  }
  story.version += 1;
  ctx.emit('story.edited', {
    subject: story.id, summary: `Story "${story.title}" edited to v${story.version} by ${ctx.user.name}`,
    data: { storyId: story.id, version: story.version },
  });
  return { message: `Story ${story.id} saved as v${story.version}.`, redirect: `/stories/${story.id}`, story };
}

function attachAsset(ctx) {
  const b = ctx.body;
  const story = find(ctx.state.stories, b.storyId);
  if (!story) throw new R.AppError(404, 'story_not_found', `Story "${b.storyId}" does not exist.`);
  const asset = find(ctx.state.assets, b.assetId);
  if (ctx.failure === 'missing_asset' || !asset) {
    throw new R.AppError(404, 'missing_asset',
      `Asset "${b.assetId}" could not be resolved in the asset repository${asset ? ' (simulated dangling reference)' : ''}.`,
      { storyId: story.id, assetId: b.assetId, simulated: ctx.failure === 'missing_asset' || undefined });
  }
  const role = ['hero', 'inline', 'video'].includes(b.role) ? b.role : 'inline';
  story.assets = story.assets.filter((a) => a.role !== role || role === 'inline');
  story.assets.push({ assetId: asset.id, role });
  ctx.emit('story.asset_attached', {
    subject: story.id, summary: `Asset ${asset.id} (${asset.name}) attached to "${story.title}" as ${role}`,
    data: { storyId: story.id, assetId: asset.id, role },
  });
  return { message: `Asset ${asset.id} attached as ${role}.`, redirect: `/stories/${story.id}`, story };
}

function submitForReview(ctx) {
  const b = ctx.body;
  const story = find(ctx.state.stories, b.storyId);
  if (!story) throw new R.AppError(404, 'story_not_found', `Story "${b.storyId}" does not exist.`);
  if (ctx.failure === 'data_conflict') {
    throw new R.AppError(409, 'data_conflict', `Story ${story.id} already moved to review by someone else (simulated).`, { storyId: story.id, simulated: true });
  }
  if (story.status !== 'draft' && story.status !== 'changes_requested') {
    throw new R.AppError(409, 'data_conflict', `Story ${story.id} is "${story.status}" — only drafts or changes-requested stories can be submitted.`, { storyId: story.id, status: story.status });
  }
  story.status = 'in_review';
  story.submittedAt = new Date().toISOString();
  ctx.emit('story.submitted_for_review', {
    subject: story.id, summary: `"${story.title}" submitted for editorial review by ${ctx.user.name}`,
    data: { storyId: story.id, version: story.version },
  });
  const n = ctx.notify(story.section === 'sports' ? 'u-2' : 'u-1', 'editorial', `Review requested: "${story.title}"`,
    `${ctx.user.name} submitted "${story.title}" (v${story.version}) for review.`);
  return {
    message: `Story ${story.id} submitted for review.`,
    warning: n && n.status === 'failed' ? 'Submission recorded, but the editor notification could not be delivered.' : null,
    redirect: `/stories/${story.id}`, story,
  };
}

function decideStory(ctx) {
  const b = ctx.body;
  const story = find(ctx.state.stories, b.storyId);
  if (!story) throw new R.AppError(404, 'story_not_found', `Story "${b.storyId}" does not exist.`);
  if (story.authorId === ctx.user.id) {
    throw new R.AppError(403, 'permission_denied',
      `Authors cannot approve their own story (self-approval blocked): ${ctx.user.username} authored "${story.title}".`,
      { storyId: story.id, reason: 'self_approval', user: ctx.user.username });
  }
  if (ctx.failure === 'data_conflict') {
    throw new R.AppError(409, 'data_conflict', `Story ${story.id} changed state while deciding (simulated).`, { storyId: story.id, simulated: true });
  }
  if (story.status !== 'in_review') {
    throw new R.AppError(409, 'data_conflict', `Story ${story.id} is "${story.status}" — only in-review stories can be decided.`, { storyId: story.id, status: story.status });
  }
  const decision = b.decision === 'request_changes' ? 'request_changes' : 'approve';
  if (decision === 'approve') {
    story.status = 'approved';
    story.approvedById = ctx.user.id;
    ctx.emit('story.approved', {
      subject: story.id, summary: `"${story.title}" approved for publication by ${ctx.user.name}`,
      data: { storyId: story.id, version: story.version },
    });
    const n = ctx.notify(story.authorId, 'editorial', `Your story "${story.title}" was approved`,
      `${ctx.user.name} approved "${story.title}" for publication.`);
    return {
      message: `Story ${story.id} approved for publication.`,
      warning: n && n.status === 'failed' ? 'Approval recorded, but the author notification could not be delivered.' : null,
      redirect: `/stories/${story.id}`, story,
    };
  }
  story.status = 'changes_requested';
  ctx.emit('story.changes_requested', {
    subject: story.id, summary: `Changes requested on "${story.title}" by ${ctx.user.name}: ${b.note || ''}`,
    data: { storyId: story.id, note: b.note || '' },
  });
  ctx.notify(story.authorId, 'editorial', `Changes requested on "${story.title}"`, `${ctx.user.name}: ${b.note || 'Please revise.'}`);
  return { message: `Changes requested on story ${story.id}.`, redirect: `/stories/${story.id}`, story };
}

function publishStory(ctx) {
  const b = ctx.body;
  const story = find(ctx.state.stories, b.storyId);
  if (!story) throw new R.AppError(404, 'story_not_found', `Story "${b.storyId}" does not exist.`);
  if (ctx.failure === 'data_conflict') {
    throw new R.AppError(409, 'data_conflict', `Story ${story.id} was published or changed by someone else while you were publishing (simulated).`, { storyId: story.id, simulated: true });
  }
  if (story.status === 'published' || story.status === 'corrected') {
    throw new R.AppError(409, 'duplicate_event', `Story ${story.id} is already published — use a correction instead.`, { storyId: story.id });
  }
  if (story.status !== 'approved') {
    throw new R.AppError(400, 'story_not_approved', `Story ${story.id} is "${story.status}" — only approved stories can be published.`, { storyId: story.id, status: story.status });
  }
  const channels = R.asList(b.channels).length ? R.asList(b.channels) : ['ch-1'];
  const hero = story.assets.find((a) => a.role === 'hero');
  const heroAsset = hero ? find(ctx.state.assets, hero.assetId) : null;
  if (ctx.failure === 'missing_asset' || !heroAsset) {
    throw new R.AppError(404, 'missing_asset',
      `Hero image could not be resolved for "${story.title}"${heroAsset ? ' (simulated dangling reference)' : ' — no hero image is attached'}. Publication blocked.`,
      { storyId: story.id, assetId: hero ? hero.assetId : null, simulated: ctx.failure === 'missing_asset' || undefined });
  }
  const jobs = [];
  const failures = [];
  for (const chId of channels) {
    const ch = find(ctx.state.channels, chId);
    if (!ch) continue;
    const stale = ctx.failure === 'stale_entitlement' || isPast(ctx, ch.licenseValidUntil);
    const job = {
      id: R.nextId(ctx.state, 'jobSeq', 'dj-'), storyId: story.id, channelId: ch.id,
      status: stale ? 'failed' : 'sent', at: new Date().toISOString(),
      error: stale ? `stale_entitlement: distribution license for channel "${ch.name}" expired ${ch.licenseValidUntil}` : null,
    };
    ctx.state.distributionJobs.push(job);
    jobs.push(job);
    if (stale) failures.push(ch.name);
    ctx.emit(stale ? 'distribution.failed' : 'distribution.sent', {
      subject: job.id, summary: `${stale ? 'FAILED' : 'Sent'} "${story.title}" to channel ${ch.name}${stale ? ' — license expired' : ''}`,
      data: { jobId: job.id, storyId: story.id, channelId: ch.id, error: job.error },
    });
  }
  story.status = 'published';
  story.publishedAt = new Date().toISOString();
  story.channels = jobs.filter((j) => j.status === 'sent').map((j) => j.channelId);
  story.version += 1;
  ctx.emit('story.published', {
    subject: story.id, summary: `"${story.title}" published to ${jobs.filter((j) => j.status === 'sent').length} channel(s) by ${ctx.user.name}`,
    data: { storyId: story.id, channels: story.channels, version: story.version },
  });
  const n = ctx.notify(story.authorId, 'editorial', `Your story "${story.title}" is live`,
    `${ctx.user.name} published "${story.title}".`);
  return {
    message: `Story ${story.id} published (${jobs.filter((j) => j.status === 'sent').length}/${jobs.length} channels delivered).`,
    warning: failures.length ? `Published, but distribution to these channels failed (stale license): ${failures.join(', ')}.` : (n && n.status === 'failed' ? 'Published, but the author notification could not be delivered.' : null),
    redirect: `/stories/${story.id}`, story, distributionJobs: jobs,
  };
}

function applyCorrection(ctx) {
  const b = ctx.body;
  const story = find(ctx.state.stories, b.storyId);
  if (!story) throw new R.AppError(404, 'story_not_found', `Story "${b.storyId}" does not exist.`);
  if (ctx.failure === 'data_conflict') {
    throw new R.AppError(409, 'data_conflict', `Story ${story.id} changed while the correction was being applied (simulated).`, { storyId: story.id, simulated: true });
  }
  checkVersion(ctx, story);
  if (story.status !== 'published' && story.status !== 'corrected') {
    throw new R.AppError(409, 'data_conflict', `Story ${story.id} is "${story.status}" — corrections apply only to published stories.`, { storyId: story.id, status: story.status });
  }
  const corr = {
    id: R.nextId(ctx.state, 'corrSeq', 'c-80'), storyId: story.id,
    summary: String(b.summary || 'Correction'), beforeText: String(b.beforeText || ''), afterText: String(b.afterText || ''),
    byId: ctx.user.id, at: new Date().toISOString(), status: 'published',
  };
  if (corr.beforeText && story.body.includes(corr.beforeText)) {
    story.body = story.body.replace(corr.beforeText, corr.afterText || corr.beforeText);
  }
  ctx.state.corrections.push(corr);
  story.status = 'corrected';
  story.version += 1;
  const jobs = [];
  for (const chId of story.channels) {
    const job = {
      id: R.nextId(ctx.state, 'jobSeq', 'dj-'), storyId: story.id, channelId: chId,
      status: 'sent', at: new Date().toISOString(), error: null, correctionId: corr.id,
    };
    ctx.state.distributionJobs.push(job);
    jobs.push(job);
    ctx.emit('distribution.sent', {
      subject: job.id, summary: `Re-sent corrected "${story.title}" to channel ${chId}`,
      data: { jobId: job.id, storyId: story.id, channelId: chId, correctionId: corr.id },
    });
  }
  ctx.emit('correction.applied', {
    subject: corr.id, summary: `Correction applied to "${story.title}" (v${story.version}) by ${ctx.user.name}: ${corr.summary}`,
    data: { storyId: story.id, correctionId: corr.id, version: story.version },
  });
  ctx.notify(story.authorId, 'editorial', `Correction applied to "${story.title}"`, `${ctx.user.name}: ${corr.summary}`);
  return {
    message: `Correction ${corr.id} applied; story re-distributed to ${jobs.length} channel(s).`,
    redirect: `/stories/${story.id}`, story, correction: corr,
  };
}

module.exports = { createStory, editStory, attachAsset, submitForReview, decideStory, publishStory, applyCorrection, today, isPast, find };
