'use strict';
/**
 * PressRoom domain operations — asset repository, renditions (image/video
 * workflow), social distribution.
 */

const R = require('../_lib/runtime');
const { find } = require('./ops');

function uploadAsset(ctx) {
  const b = ctx.body;
  const kind = ['image', 'video', 'audio'].includes(b.kind) ? b.kind : 'image';
  const asset = {
    id: R.nextId(ctx.state, 'assetSeq', 'a-'), kind, name: String(b.name || 'untitled'),
    sizeMb: Math.round(parseFloat(b.sizeMb) * 10) / 10 || 1.0,
    width: parseInt(b.width, 10) || 0, height: parseInt(b.height, 10) || 0,
    credit: String(b.credit || 'Staff'), license: ['editorial', 'handout', 'purchased'].includes(b.license) ? b.license : 'editorial',
    renditions: [], checksum: Math.random().toString(16).slice(2, 10), uploadedById: ctx.user.id,
  };
  ctx.state.assets.push(asset);
  ctx.emit('asset.uploaded', {
    subject: asset.id, summary: `Asset ${asset.id} "${asset.name}" (${kind}, ${asset.sizeMb} MB) uploaded to the repository`,
    data: { assetId: asset.id, kind },
  });
  return { message: `Asset ${asset.id} uploaded.`, redirect: '/assets', asset };
}

function requestRendition(ctx) {
  const b = ctx.body;
  const asset = find(ctx.state.assets, b.assetId);
  if (!asset) throw new R.AppError(404, 'asset_not_found', `Asset "${b.assetId}" does not exist.`);
  if (ctx.failure === 'data_conflict') {
    throw new R.AppError(409, 'data_conflict', `Rendition request conflicts with an in-flight job for ${asset.id} (simulated).`, { assetId: asset.id, simulated: true });
  }
  const name = String(b.rendition || 'thumbnail');
  if (asset.renditions.find((r) => r.name === name && r.status === 'processing')) {
    throw new R.AppError(409, 'data_conflict', `Rendition "${name}" for ${asset.id} is already processing.`, { assetId: asset.id, rendition: name });
  }
  const dimensions = name === 'thumbnail' ? '320w' : name === 'social-card' ? '1200x630' : name === 'hero' ? '2560w' : 'native';
  const rend = { name, dimensions, status: 'ready' };
  asset.renditions = asset.renditions.filter((r) => r.name !== name);
  asset.renditions.push(rend);
  ctx.emit('rendition.ready', {
    subject: asset.id, summary: `Rendition "${name}" (${dimensions}) ready for asset ${asset.name}`,
    data: { assetId: asset.id, rendition: name },
  });
  return { message: `Rendition "${name}" rendered for ${asset.id}.`, redirect: '/assets', asset };
}

function scheduleSocialPost(ctx) {
  const b = ctx.body;
  const story = find(ctx.state.stories, b.storyId);
  if (!story) throw new R.AppError(404, 'story_not_found', `Story "${b.storyId}" does not exist.`);
  const network = ['feedbook', 'chirp', 'linkup'].includes(b.network) ? b.network : 'feedbook';
  if (ctx.state.socialPosts.find((p) => p.storyId === story.id && p.network === network && ['scheduled', 'posted'].includes(p.status))) {
    throw new R.AppError(409, 'duplicate_event',
      `A ${network} post for "${story.title}" is already scheduled or posted.`,
      { storyId: story.id, network });
  }
  const post = {
    id: R.nextId(ctx.state, 'socialSeq', 'sp-'), storyId: story.id, network,
    text: String(b.text || story.title), status: 'scheduled',
    scheduledAt: String(b.scheduledAt || new Date().toISOString()), postedAt: null, engagement: null,
  };
  ctx.state.socialPosts.push(post);
  story.socialPosts.push(post.id);
  ctx.emit('social.scheduled', {
    subject: post.id, summary: `${network} post scheduled for "${story.title}" (${post.scheduledAt}) by ${ctx.user.name}`,
    data: { postId: post.id, storyId: story.id, network, scheduledAt: post.scheduledAt },
  });
  const n = ctx.notify('u-1', 'social', `${network} post scheduled for "${story.title}"`,
    `${ctx.user.name} scheduled: "${post.text.slice(0, 80)}"`);
  return {
    message: `Social post ${post.id} scheduled on ${network}.`,
    warning: n && n.status === 'failed' ? 'Post scheduled, but the editor notification could not be delivered.' : null,
    redirect: '/social', post,
  };
}

function publishSocialPost(ctx) {
  const b = ctx.body;
  const post = find(ctx.state.socialPosts, b.postId);
  if (!post) throw new R.AppError(404, 'post_not_found', `Social post "${b.postId}" does not exist.`);
  if (ctx.failure === 'data_conflict') {
    throw new R.AppError(409, 'data_conflict', `Post ${post.id} was already published by someone else (simulated).`, { postId: post.id, simulated: true });
  }
  if (post.status === 'posted') {
    throw new R.AppError(409, 'duplicate_event', `Post ${post.id} was already published.`, { postId: post.id });
  }
  const failed = ctx.failure === 'notification_failure'; // platform delivery failure
  if (failed) {
    post.status = 'failed';
    ctx.emit('social.post_failed', {
      subject: post.id, summary: `${post.network} post for story ${post.storyId} FAILED to deliver (simulated platform outage)`,
      data: { postId: post.id, network: post.network },
    });
    return {
      message: `Post ${post.id} could not be delivered to ${post.network}.`,
      warning: `The ${post.network} delivery failed — the post is marked failed and can be re-scheduled.`,
      redirect: '/social', post,
    };
  }
  post.status = 'posted';
  post.postedAt = new Date().toISOString();
  post.engagement = { likes: 0, reposts: 0, clicks: 0 };
  ctx.emit('social.posted', {
    subject: post.id, summary: `${post.network} post published for story ${post.storyId} by ${ctx.user.name}`,
    data: { postId: post.id, network: post.network },
  });
  return { message: `Post ${post.id} published to ${post.network}.`, redirect: '/social', post };
}

module.exports = { uploadAsset, requestRendition, scheduleSocialPost, publishSocialPost };
