'use strict';
/**
 * PressRoom server-rendered UI — dashboard, desk (story list), story detail with
 * the full editorial workflow forms, asset repository, social dashboard.
 */

const W = require('../_lib/web');
const R = require('../_lib/runtime');
const esc = W.esc;

function userName(s, id) {
  const u = R.findUserById(s, id);
  return u ? u.name : (id || '—');
}
function table(headers, arr, rowFn) { return W.tbl(headers, arr.map(rowFn).join('')); }
function can(ctx, perm) { return ctx.user && ctx.user.permissions.includes(perm); }

function dashboard(ctx) {
  const s = ctx.state;
  const byStatus = {};
  for (const st of s.stories) byStatus[st.status] = (byStatus[st.status] || 0) + 1;
  const failedJobs = s.distributionJobs.filter((j) => j.status === 'failed');
  const failedNotifs = ctx.user ? s.notifications.filter((n) => n.userId === ctx.user.id && n.status === 'failed').length : 0;
  const storiesTable = table(['Story', 'Section', 'Author', 'Status', 'Version', 'Published'], s.stories, (st) => W.tr([
    `<a href="/stories/${esc(st.id)}"><b>${esc(st.title)}</b></a>`, esc(st.section), esc(userName(s, st.authorId)),
    W.statusPill(st.status), `v${st.version}`, st.publishedAt ? `<span class="mono">${esc(st.publishedAt)}</span>` : '—',
  ]));
  return W.page(ctx.app.config, {
    title: 'Dashboard', user: ctx.user, query: ctx.query,
    body: `
<h1>PressRoom — editorial &amp; distribution</h1>
${ctx.user ? '' : '<div class="flash flash-warn" role="alert">Public view. <a href="/login">Sign in</a> as a seeded persona (editor-in-chief, section editor, writer, photo editor, social manager) to operate.</div>'}
<div class="grid grid3">
${W.kpi('Drafts', byStatus.draft || 0, 'cms')}
${W.kpi('In review', byStatus.in_review || 0, 'awaiting editors')}
${W.kpi('Approved, unpublished', byStatus.approved || 0, 'ready to publish')}
${W.kpi('Published / corrected', (byStatus.published || 0) + (byStatus.corrected || 0), 'live')}
${W.kpi('Failed distributions', failedJobs.length, 'stale licenses etc.')}
${W.kpi('Your failed notifications', failedNotifs, 'notification_failure')}
</div>
${W.card('Story pipeline', storiesTable)}
<p class="muted"><a href="/desk">Editorial desk</a> · <a href="/assets">Asset repository</a> · <a href="/social">Social dashboard</a> · <a href="/failures">failure switches</a> · repository: <code>docs/validation/fixtures/media/</code>.</p>`,
  });
}

function desk(ctx) {
  const s = ctx.state;
  const canCreate = can(ctx, 'story:create');
  const createForm = canCreate ? W.card('Create a story', W.frm('/ui/stories/create', [
    W.fld('Title', W.txt('title', '', { required: true })),
    W.fld('Section', W.sel('section', s.sections.map((x) => [x, x]))),
    W.fld('Dek (subtitle)', W.txt('dek', '')),
    W.fld('Body', W.ta('body', '')),
  ], { submit: 'Create draft' })) : '';
  const rows = s.stories.map((st) => W.tr([
    `<a href="/stories/${esc(st.id)}"><b>${esc(st.title)}</b></a>`, `<code>${esc(st.id)}</code>`, esc(st.section),
    esc(userName(s, st.authorId)), W.statusPill(st.status), `v${st.version}`, esc(st.dek),
  ])).join('');
  return W.page(ctx.app.config, {
    title: 'Desk', user: ctx.user, query: ctx.query,
    body: `<h1>Editorial desk</h1>${createForm}
${W.card('All stories', W.tbl(['Story', 'ID', 'Section', 'Author', 'Status', 'Version', 'Dek'], rows))}`,
  });
}

function storyPage(ctx) {
  const s = ctx.state;
  const story = s.stories.find((x) => x.id === ctx.params.id);
  if (!story) throw new R.AppError(404, 'story_not_found', `Story "${ctx.params.id}" does not exist.`);
  const isAuthor = ctx.user && ctx.user.id === story.authorId;
  const isEditor = ctx.user && ctx.user.permissions.includes('story:approve');
  const canEdit = ctx.user && (isAuthor || isEditor) && ['draft', 'changes_requested', 'in_review'].includes(story.status);
  const canSubmit = isAuthor && ['draft', 'changes_requested'].includes(story.status);
  const canDecide = isEditor && story.status === 'in_review' && !isAuthor;
  const canPublish = ctx.user && ctx.user.permissions.includes('story:publish') && story.status === 'approved';
  const canCorrect = isEditor && ['published', 'corrected'].includes(story.status);
  const jobs = s.distributionJobs.filter((j) => j.storyId === story.id).slice().reverse();
  const posts = story.socialPosts.map((pid) => s.socialPosts.find((p) => p.id === pid)).filter(Boolean);
  const corrs = s.corrections.filter((c) => c.storyId === story.id).slice().reverse();

  const overview = W.dl([
    ['ID', `<code>${esc(story.id)}</code>`], ['Slug', `<code>${esc(story.slug)}</code>`],
    ['Section', esc(story.section)], ['Author', esc(userName(s, story.authorId))],
    ['Status', W.statusPill(story.status)], ['Version', `v${story.version}`],
    ['Word count', story.wordCount], ['Approved by', story.approvedById ? esc(userName(s, story.approvedById)) : '—'],
    ['Published at', story.publishedAt ? `<span class="mono">${esc(story.publishedAt)}</span>` : '—'],
    ['Channels', story.channels.map((c) => `<code>${esc(channelName(s, c))}</code>`).join(', ') || '—'],
  ]);
  const assetsTable = story.assets.length ? table(['Asset', 'Kind', 'Name', 'Credit', 'Renditions'], story.assets, (a) => {
    const asset = s.assets.find((x) => x.id === a.assetId) || {};
    return W.tr([`<code>${esc(a.assetId)}</code> ${W.pill('info', a.role)}`, esc(asset.kind || '?'), esc(asset.name || '?'), esc(asset.credit || ''),
      (asset.renditions || []).map((r) => `${esc(r.name)}:${W.statusPill(r.status)}`).join(' ')]);
  }) : '<p class="muted">No assets attached (a hero image is required to publish).</p>';
  const attachForm = can(ctx, 'story:edit') && ['draft', 'changes_requested', 'in_review'].includes(story.status) ? W.frm('/ui/stories/attach', [
    W.hidden('storyId', story.id),
    W.fld('Asset', W.sel('assetId', s.assets.map((a) => [a.id, `${a.id} — ${a.name} (${a.kind})`]))),
    W.fld('Role', W.sel('role', [['hero', 'Hero (required to publish)'], ['inline', 'Inline'], ['video', 'Video']])),
  ], { submit: 'Attach asset', note: 'missing_asset: attaching an unknown id fails with 404.' }) : '';
  const editForm = canEdit ? W.frm('/ui/stories/edit', [
    W.hidden('storyId', story.id),
    W.hidden('expectedVersion', story.version),
    W.fld('Title', W.txt('title', story.title)),
    W.fld('Dek', W.txt('dek', story.dek)),
    W.fld('Body (v' + story.version + ')', W.ta('body', story.body, 8)),
  ], { submit: 'Save new version', note: 'Optimistic concurrency: saving a stale version returns data_conflict.' }) : '';
  const submitForm = canSubmit ? W.frm('/ui/stories/submit', [W.hidden('storyId', story.id)], { submit: 'Submit for review' }) : '';
  const decideForm = canDecide ? W.frm('/ui/stories/decide', [
    W.hidden('storyId', story.id),
    W.fld('Decision', W.sel('decision', [['approve', 'Approve for publication'], ['request_changes', 'Request changes']])),
    W.fld('Note to author', W.txt('note', '')),
  ], { submit: 'Decide', note: 'Self-approval is blocked by policy.' }) : '';
  const publishForm = canPublish ? W.frm('/ui/stories/publish', [
    W.hidden('storyId', story.id),
    W.fld('Channels', W.checks('channels', s.channels.map((c) => [c.id, `${c.name}${isPast(ctx, c.licenseValidUntil) ? ' ⚠ license expired' : ''}`, c.id === 'ch-1']))),
  ], { submit: 'Publish', note: 'Requires a hero image. Channels with expired licenses fail with stale_entitlement (seed: partner-app).' }) : '';
  const correctForm = canCorrect ? W.frm('/ui/stories/correct', [
    W.hidden('storyId', story.id),
    W.hidden('expectedVersion', story.version),
    W.fld('Correction summary', W.txt('summary', '', { required: true })),
    W.fld('Replace text (before)', W.txt('beforeText', '')),
    W.fld('With (after)', W.txt('afterText', '')),
  ], { submit: 'Apply correction', note: 'Applies the correction, bumps the version and re-distributes to the story channels.' }) : '';
  const jobsTable = jobs.length ? table(['Job', 'Channel', 'Status', 'At', 'Error'], jobs, (j) => W.tr([
    `<code>${esc(j.id)}</code>`, `<code>${esc(channelName(s, j.channelId))}</code>`, W.statusPill(j.status),
    `<span class="mono">${esc(j.at)}</span>`, j.error ? `<span class="muted">${esc(j.error)}</span>` : '—',
  ])) : '<p class="muted">No distribution jobs.</p>';
  const postsTable = posts.length ? table(['Post', 'Network', 'Text', 'Status', 'Engagement'], posts, (p) => W.tr([
    `<code>${esc(p.id)}</code>`, esc(p.network), esc(p.text), W.statusPill(p.status),
    p.engagement ? `${p.engagement.likes} likes · ${p.engagement.reposts} reposts · ${p.engagement.clicks} clicks` : '—',
  ])) : '<p class="muted">No social posts.</p>';
  const corrTable = corrs.length ? table(['Correction', 'Summary', 'Before', 'After', 'By', 'At'], corrs, (c) => W.tr([
    `<code>${esc(c.id)}</code>`, esc(c.summary), `<code>${esc(c.beforeText)}</code>`, `<code>${esc(c.afterText)}</code>`,
    esc(userName(s, c.byId)), `<span class="mono">${esc(c.at)}</span>`,
  ])) : '<p class="muted">No corrections.</p>';
  const socialForm = can(ctx, 'social:schedule') && ['published', 'corrected'].includes(story.status) ? W.frm('/ui/social/schedule', [
    W.hidden('storyId', story.id),
    W.fld('Network', W.sel('network', [['feedbook', 'Feedbook'], ['chirp', 'Chirp'], ['linkup', 'Linkup']])),
    W.fld('Post text', W.txt('text', story.title)),
    W.fld('Scheduled at', W.txt('scheduledAt', '2026-09-13T09:00:00Z')),
  ], { submit: 'Schedule post' }) : '';
  return W.page(ctx.app.config, {
    title: story.title, user: ctx.user, query: ctx.query,
    body: `
<h1>${esc(story.title)} ${W.statusPill(story.status)}</h1>
${W.card('Story overview', overview)}
${W.card('Body', `<p style="white-space:pre-wrap">${esc(story.body)}</p>`)}
${W.card('Assets', assetsTable + attachForm)}
${W.card('Edit', editForm)}
${W.card('Submit for review', submitForm)}
${W.card('Editorial decision', decideForm)}
${W.card('Publish', publishForm)}
${W.card('Corrections', corrTable + correctForm)}
${W.card('Distribution', jobsTable)}
${W.card('Social posts', postsTable + socialForm)}`,
  });
}
function channelName(s, id) {
  const c = (s.channels || []).find((x) => x.id === id);
  return c ? c.name : id;
}
function isPast(ctx, d) { return String(d || '9999-12-31') < (ctx.state.meta.today || '2026-09-12'); }

function assets(ctx) {
  const s = ctx.state;
  const canUpload = can(ctx, 'asset:upload');
  const canRend = can(ctx, 'rendition:request');
  const rows = s.assets.map((a) => W.tr([
    `<code>${esc(a.id)}</code>`, W.statusPill(a.kind), esc(a.name), `${a.sizeMb} MB`, `${a.width}×${a.height}`,
    esc(a.credit), W.statusPill(a.license),
    a.renditions.map((r) => `${esc(r.name)}:${W.statusPill(r.status)}`).join(' ') || '—',
    canRend ? W.frm('/ui/assets/rendition', [
      W.hidden('assetId', a.id),
      W.fld('Rendition', W.sel('rendition', ['thumbnail', 'social-card', 'hero', 'web-1080p'])),
    ], { submit: 'Render' }) : '—',
  ])).join('');
  const uploadForm = canUpload ? W.card('Upload asset', W.frm('/ui/assets/upload', [
    W.fld('Kind', W.sel('kind', [['image', 'Image'], ['video', 'Video'], ['audio', 'Audio']])),
    W.fld('File name', W.txt('name', 'city-hall-presser.jpg', { required: true })),
    W.fld('Size (MB)', W.txt('sizeMb', '2.5', { type: 'number', step: '0.1', min: '0.1' })),
    W.fld('Width', W.txt('width', '3840', { type: 'number' })),
    W.fld('Height', W.txt('height', '2560', { type: 'number' })),
    W.fld('Credit', W.txt('credit', 'Staff')),
    W.fld('License', W.sel('license', [['editorial', 'Editorial'], ['handout', 'Handout'], ['purchased', 'Purchased']])),
  ], { submit: 'Upload' })) : '';
  return W.page(ctx.app.config, {
    title: 'Assets', user: ctx.user, query: ctx.query,
    body: `<h1>Asset repository</h1>
${W.card('All assets', W.tbl(['Asset', 'Kind', 'Name', 'Size', 'Dimensions', 'Credit', 'License', 'Renditions', 'Render'], rows))}
${uploadForm}`,
  });
}

function social(ctx) {
  const s = ctx.state;
  const canSchedule = can(ctx, 'social:schedule');
  const canPost = can(ctx, 'social:post');
  const rows = s.socialPosts.slice().reverse().map((p) => W.tr([
    `<code>${esc(p.id)}</code>`, storyLink(s, p.storyId), esc(p.network), esc(p.text), W.statusPill(p.status),
    `<span class="mono">${esc(p.scheduledAt)}</span>`,
    p.engagement ? `${p.engagement.likes} / ${p.engagement.reposts} / ${p.engagement.clicks}` : '—',
    canPost && p.status === 'scheduled' ? W.frm('/ui/social/publish', [W.hidden('postId', p.id)], { submit: 'Post now' }) : '—',
  ])).join('');
  const scheduleForm = canSchedule ? W.card('Schedule a social post', W.frm('/ui/social/schedule', [
    W.fld('Story', W.sel('storyId', s.stories.filter((st) => ['published', 'corrected'].includes(st.status)).map((st) => [st.id, st.title]) || [['', '— no published stories —']])),
    W.fld('Network', W.sel('network', [['feedbook', 'Feedbook'], ['chirp', 'Chirp'], ['linkup', 'Linkup']])),
    W.fld('Post text', W.txt('text', '')),
    W.fld('Scheduled at', W.txt('scheduledAt', '2026-09-13T09:00:00Z')),
  ], { submit: 'Schedule' })) : '';
  return W.page(ctx.app.config, {
    title: 'Social', user: ctx.user, query: ctx.query,
    body: `<h1>Social distribution dashboard</h1>
${W.card('Posts', W.tbl(['Post', 'Story', 'Network', 'Text', 'Status', 'Scheduled', 'Likes/Reposts/Clicks', 'Action'], rows))}
${scheduleForm}`,
  });
}
function storyLink(s, id) {
  const st = (s.stories || []).find((x) => x.id === id);
  return st ? `<a href="/stories/${esc(st.id)}">${esc(st.title)}</a>` : id;
}

function channels(ctx) {
  const s = ctx.state;
  const rows = s.channels.map((c) => W.tr([
    `<code>${esc(c.id)}</code>`, `<b>${esc(c.name)}</b>`, esc(c.kind),
    c.licenseValidUntil ? (isPast(ctx, c.licenseValidUntil) ? `${esc(c.licenseValidUntil)} ${W.pill('err', 'expired')}` : `${esc(c.licenseValidUntil)} ${W.pill('ok', 'valid')}`)
      : W.pill('ok', 'unrestricted'),
    String(s.distributionJobs.filter((j) => j.channelId === c.id).length),
  ])).join('');
  return W.page(ctx.app.config, {
    title: 'Channels', user: ctx.user, query: ctx.query,
    body: `<h1>Distribution channels</h1>
${W.card('Channel licenses', W.tbl(['ID', 'Channel', 'Kind', 'License valid until', 'Jobs delivered'], rows))}
<p class="muted">Publishing to a channel with an expired license records a failed distribution job (stale_entitlement) instead of blocking the whole publication.</p>`,
  });
}

function notifications(ctx) {
  const s = ctx.state;
  R.requireUser(ctx);
  const mine = s.notifications.filter((n) => n.userId === ctx.user.id).slice().reverse();
  const rows = mine.map((n) => W.tr([
    `<code>${esc(n.id)}</code>`, esc(n.type), `<b>${esc(n.subject)}</b>`, esc(n.text), W.statusPill(n.status),
  ])).join('');
  return W.page(ctx.app.config, {
    title: 'Notifications', user: ctx.user, query: ctx.query,
    body: `<h1>Your notifications</h1>${W.card('Inbox', mine.length ? W.tbl(['ID', 'Type', 'Subject', 'Text', 'Status'], rows) : '<p class="muted">No notifications.</p>')}`,
  });
}

module.exports = { dashboard, desk, story: storyPage, assets, social, channels, notifications };
