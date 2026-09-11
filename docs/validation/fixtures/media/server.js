'use strict';
/**
 * PressRoom — media family entry point (VWO-002 synthetic fixture).
 * Run:  node server.js [--port 4104]     (bun server.js works identically)
 */

const R = require('../_lib/runtime');
const seed = require('./seed');
const ops = require('./ops');
const dist = require('./ops-dist');
const ui = require('./ui');

const portArg = process.argv.includes('--port') ? process.argv[process.argv.indexOf('--port') + 1] : null;
const port = parseInt(portArg || process.env.FIXTURE_PORT || '4104', 10);

const failures = [
  { switch: 'service_unavailable', applies_to: 'any state-changing request', behavior: 'The synthetic "media-asset-pipeline / CMS persistence" dependency is treated as down.', symptom: 'HTTP 503 {error:"service_unavailable", service:"media-asset-pipeline"}; reads still work.' },
  { switch: 'notification_failure', applies_to: 'submit-for-review, decision, publish, correction, social posting', behavior: 'Operation succeeds; the notification record (or social platform delivery) is stored "failed".', symptom: 'HTTP 200 + warning; failed row in /notifications; notification.failed event; social post marked failed.' },
  { switch: 'missing_asset', applies_to: 'attach asset, publish story', behavior: 'A referenced asset (or the hero image) is treated as unresolvable.', symptom: 'HTTP 404 {error:"missing_asset"}; natural case: publishing a story with no hero image attached.' },
  { switch: 'permission_denied', applies_to: 'any endpoint declaring a permission', behavior: 'Permission check fails even for authorized users.', symptom: 'HTTP 403 {error:"permission_denied", simulated:true}; natural case: a writer approving their own story (self-approval policy), a writer publishing.' },
  { switch: 'data_conflict', applies_to: 'edit, submit, decide, publish, correction, rendition, social publish', behavior: 'Optimistic-concurrency mismatch / record concurrently changed.', symptom: 'HTTP 409 {error:"data_conflict", currentVersion}; natural case: saving an edit from a stale expectedVersion.' },
  { switch: 'duplicate_event', applies_to: 'any state-changing request', behavior: 'Request signature recorded + rejected as replay; idempotencyKey replays rejected.', symptom: 'HTTP 409 {error:"duplicate_event"}; natural cases: publishing twice, scheduling two posts for the same story+network, duplicate story title.' },
  { switch: 'stale_entitlement', applies_to: 'publish story to licensed channels', behavior: 'A channel distribution license is treated as expired; that channel job fails while the story still publishes to valid channels.', symptom: 'HTTP 200 + warning naming failed channels; failed distribution job with error "stale_entitlement"; natural case: publishing to partner-app (license expired 2026-06-01).' },
];

const routes = [
  { kind: 'page', method: 'GET', pattern: '/', auth: false, handler: ui.dashboard },
  { kind: 'page', method: 'GET', pattern: '/desk', auth: false, handler: ui.desk },
  { kind: 'page', method: 'GET', pattern: '/stories/:id', auth: false, handler: ui.story },
  { kind: 'page', method: 'GET', pattern: '/assets', auth: false, handler: ui.assets },
  { kind: 'page', method: 'GET', pattern: '/social', auth: false, handler: ui.social },
  { kind: 'page', method: 'GET', pattern: '/channels', auth: false, handler: ui.channels },
  { kind: 'page', method: 'GET', pattern: '/notifications', handler: ui.notifications },

  // read APIs
  { kind: 'api', method: 'GET', pattern: '/api/stories', auth: false, op: (ctx) => ({ stories: ctx.state.stories }) },
  { kind: 'api', method: 'GET', pattern: '/api/stories/:id', auth: false, op: (ctx) => {
    const st = ctx.state.stories.find((x) => x.id === ctx.params.id);
    if (!st) throw new R.AppError(404, 'story_not_found', `Story "${ctx.params.id}" does not exist.`);
    return {
      story: st,
      distributionJobs: ctx.state.distributionJobs.filter((j) => j.storyId === st.id),
      socialPosts: st.socialPosts.map((pid) => ctx.state.socialPosts.find((p) => p.id === pid)).filter(Boolean),
      corrections: ctx.state.corrections.filter((c) => c.storyId === st.id),
    };
  } },
  { kind: 'api', method: 'GET', pattern: '/api/assets', auth: false, op: (ctx) => ({ assets: ctx.state.assets }) },
  { kind: 'api', method: 'GET', pattern: '/api/channels', auth: false, op: (ctx) => ({ channels: ctx.state.channels }) },
  { kind: 'api', method: 'GET', pattern: '/api/distribution', auth: false, op: (ctx) => ({ distributionJobs: ctx.state.distributionJobs.slice().reverse() }) },
  { kind: 'api', method: 'GET', pattern: '/api/social', auth: false, op: (ctx) => ({ socialPosts: ctx.state.socialPosts }) },
  { kind: 'api', method: 'GET', pattern: '/api/corrections', auth: false, op: (ctx) => ({ corrections: ctx.state.corrections }) },
  { kind: 'api', method: 'GET', pattern: '/api/notifications', op: (ctx) => ({
    notifications: ctx.state.notifications.filter((n) => n.userId === ctx.user.id).slice().reverse(),
  }) },

  // mutations — API twins + HTML forms share ops
  { kind: 'api', method: 'POST', pattern: '/api/stories/create', perm: 'story:create',
    naturalKey: (ctx) => `title:${ctx.body.title}`, op: ops.createStory },
  { kind: 'form', method: 'POST', pattern: '/ui/stories/create', perm: 'story:create',
    naturalKey: (ctx) => `title:${ctx.body.title}`, op: ops.createStory, back: '/desk' },
  { kind: 'api', method: 'POST', pattern: '/api/stories/edit', perm: 'story:edit', op: ops.editStory },
  { kind: 'form', method: 'POST', pattern: '/ui/stories/edit', perm: 'story:edit', op: ops.editStory, back: '/desk' },
  { kind: 'api', method: 'POST', pattern: '/api/stories/attach', perm: 'story:edit', op: ops.attachAsset },
  { kind: 'form', method: 'POST', pattern: '/ui/stories/attach', perm: 'story:edit', op: ops.attachAsset, back: '/desk' },
  { kind: 'api', method: 'POST', pattern: '/api/stories/submit', perm: 'story:edit', op: ops.submitForReview },
  { kind: 'form', method: 'POST', pattern: '/ui/stories/submit', perm: 'story:edit', op: ops.submitForReview, back: '/desk' },
  { kind: 'api', method: 'POST', pattern: '/api/stories/decide', perm: 'story:approve', op: ops.decideStory },
  { kind: 'form', method: 'POST', pattern: '/ui/stories/decide', perm: 'story:approve', op: ops.decideStory, back: '/desk' },
  { kind: 'api', method: 'POST', pattern: '/api/stories/publish', perm: 'story:publish', op: ops.publishStory },
  { kind: 'form', method: 'POST', pattern: '/ui/stories/publish', perm: 'story:publish', op: ops.publishStory, back: '/desk' },
  { kind: 'api', method: 'POST', pattern: '/api/stories/correct', perm: 'correction:apply', op: ops.applyCorrection },
  { kind: 'form', method: 'POST', pattern: '/ui/stories/correct', perm: 'correction:apply', op: ops.applyCorrection, back: '/desk' },
  { kind: 'api', method: 'POST', pattern: '/api/assets/upload', perm: 'asset:upload', op: dist.uploadAsset },
  { kind: 'form', method: 'POST', pattern: '/ui/assets/upload', perm: 'asset:upload', op: dist.uploadAsset, back: '/assets' },
  { kind: 'api', method: 'POST', pattern: '/api/assets/rendition', perm: 'rendition:request', op: dist.requestRendition },
  { kind: 'form', method: 'POST', pattern: '/ui/assets/rendition', perm: 'rendition:request', op: dist.requestRendition, back: '/assets' },
  { kind: 'api', method: 'POST', pattern: '/api/social/schedule', perm: 'social:schedule',
    naturalKey: (ctx) => `post:${ctx.body.storyId}:${ctx.body.network}`, op: dist.scheduleSocialPost },
  { kind: 'form', method: 'POST', pattern: '/ui/social/schedule', perm: 'social:schedule',
    naturalKey: (ctx) => `post:${ctx.body.storyId}:${ctx.body.network}`, op: dist.scheduleSocialPost, back: '/social' },
  { kind: 'api', method: 'POST', pattern: '/api/social/publish', perm: 'social:post', op: dist.publishSocialPost },
  { kind: 'form', method: 'POST', pattern: '/ui/social/publish', perm: 'social:post', op: dist.publishSocialPost, back: '/social' },
];

const app = R.createApp({
  family: 'media',
  appName: 'pressroom',
  appTitle: 'PressRoom',
  tagline: 'Editorial CMS · assets · approvals · distribution — synthetic validation fixture',
  port,
  rootDir: __dirname,
  seedFn: seed.build,
  serviceName: 'media-asset-pipeline',
  accent: '#7c3aed', accentSoft: '#f1e9fd',
  nav: [['/', 'Dashboard'], ['/desk', 'Desk'], ['/assets', 'Assets'], ['/social', 'Social'], ['/channels', 'Channels'], ['/notifications', 'Notifications'], ['/events', 'Events'], ['/failures', 'Failure switches']],
  failures,
  routes,
});

app.start();
