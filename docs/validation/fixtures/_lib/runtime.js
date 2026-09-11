'use strict';
/**
 * VWO-002 synthetic fixture runtime — shared by all five application families.
 *
 * Std-lib only (node:http / node:fs / node:path / node:crypto). Runs under bun or node >= 18.
 * This is disposable validation infrastructure for the human-workflow validation program:
 * it is NOT a workflow engine and deliberately holds no workflow semantics. It only serves
 * small stateful demo apps with seeded users, JSON-file state, an event feed and
 * deterministic per-request failure switches.
 */

const http = require('node:http');
const fs = require('node:fs');
const path = require('node:path');

const FAILURE_SWITCHES = [
  'service_unavailable',
  'notification_failure',
  'missing_asset',
  'permission_denied',
  'data_conflict',
  'duplicate_event',
  'stale_entitlement',
];
const FAILURE_ALIASES = { notification_down: 'notification_failure' };
const MAX_BODY_BYTES = 1024 * 1024;
const MAX_EVENTS = 1000;

class AppError extends Error {
  constructor(status, code, message, extra) {
    super(message);
    this.status = status;
    this.code = code;
    this.extra = extra || {};
  }
  toJSON() {
    return Object.assign({ ok: false, error: this.code, message: this.message }, this.extra);
  }
}

// ---------------------------------------------------------------- store

function createStore(rootDir, seedFn) {
  const dataDir = path.join(rootDir, 'data');
  const statePath = path.join(dataDir, 'state.json');
  let state = null;

  function ensureMeta(s) {
    s.meta = Object.assign({ eventSeq: 0, notifSeq: 0, resets: 0, idempotency: {} }, s.meta || {});
    return s;
  }
  function seed() {
    state = ensureMeta(seedFn());
    if (!Array.isArray(state.users)) throw new Error('seed must define state.users[]');
    if (!Array.isArray(state.notifications)) state.notifications = [];
    if (!Array.isArray(state.events)) state.events = [];
    persist();
    return state;
  }
  function load() {
    if (state) return state;
    try {
      state = ensureMeta(JSON.parse(fs.readFileSync(statePath, 'utf8')));
      if (!Array.isArray(state.users)) throw new Error('corrupt state');
    } catch {
      seed();
    }
    return state;
  }
  function persist() {
    fs.mkdirSync(dataDir, { recursive: true });
    const tmp = statePath + '.tmp';
    fs.writeFileSync(tmp, JSON.stringify(state, null, 2));
    fs.renameSync(tmp, statePath);
  }
  return {
    get state() { return load(); },
    save: persist,
    reset() { state = null; return seed(); },
  };
}

// ---------------------------------------------------------------- auth

function assemble(parts) { return parts.join('-'); }

function publicUser(u) {
  return { id: u.id, username: u.username, name: u.name, role: u.role, permissions: u.permissions.slice() };
}
function findUserByUsername(state, username) {
  return state.users.find((u) => u.username === username) || null;
}
function findUserByToken(state, token) {
  return state.users.find((u) => assemble(u.tokenParts) === token) || null;
}
function findUserById(state, id) {
  return state.users.find((u) => u.id === id) || null;
}
function demoHints(state) {
  return state.users.map((u) => ({
    username: u.username, password: assemble(u.passwordParts), name: u.name, role: u.role,
  }));
}
function checkLogin(state, username, password) {
  const u = findUserByUsername(state, username);
  if (!u || assemble(u.passwordParts) !== String(password)) {
    throw new AppError(401, 'invalid_credentials', 'Unknown user or wrong password (demo credentials are listed on the login page).');
  }
  return { user: publicUser(u), token: assemble(u.tokenParts) };
}
function requireUser(ctx) {
  if (!ctx.user) throw new AppError(401, 'authentication_required', 'Sign in to perform this operation.');
  return ctx.user;
}
function requirePermission(ctx, perm) {
  const u = requireUser(ctx);
  if (ctx.failure === 'permission_denied') {
    throw new AppError(403, 'permission_denied',
      `Permission denied (simulated): "${perm}" is required and the check failed while the failure switch is on.`,
      { simulated: true, required: perm, role: u.role, user: u.username });
  }
  if (!u.permissions.includes(perm)) {
    throw new AppError(403, 'permission_denied',
      `User "${u.username}" (role "${u.role}") lacks permission "${perm}".`,
      { required: perm, role: u.role, user: u.username });
  }
  return u;
}

// ---------------------------------------------------------------- events + notifications

function nextId(state, field, prefix) {
  state.meta[field] = (state.meta[field] || 0) + 1;
  return `${prefix}${String(state.meta[field]).padStart(4, '0')}`;
}
function pushEvent(state, actor, type, fields) {
  const ev = Object.assign({
    id: nextId(state, 'eventSeq', 'ev-'),
    ts: new Date().toISOString(),
    type,
    actor: actor || 'system',
  }, fields || {});
  state.events.push(ev);
  if (state.events.length > MAX_EVENTS) state.events.splice(0, state.events.length - MAX_EVENTS);
  return ev;
}
function emit(ctx, type, fields) {
  return pushEvent(ctx.state, ctx.user ? ctx.user.username : 'system', type, fields);
}
function notify(ctx, userId, type, subject, text) {
  const target = findUserById(ctx.state, userId);
  if (!target) return null;
  const failed = ctx.failure === 'notification_failure';
  const n = {
    id: nextId(ctx.state, 'notifSeq', 'n-'),
    userId: target.id, to: target.username, type, subject, text,
    status: failed ? 'failed' : 'delivered',
    createdAt: new Date().toISOString(),
  };
  ctx.state.notifications.push(n);
  const actor = ctx.user ? ctx.user.username : 'system';
  if (failed) {
    pushEvent(ctx.state, actor, 'notification.failed', {
      subject: `notification:${n.id}`,
      summary: `Notification "${subject}" to ${target.username} FAILED to deliver (simulated switch)`,
      data: { notificationId: n.id, to: target.username, type },
    });
  } else {
    pushEvent(ctx.state, actor, 'notification.delivered', {
      subject: `notification:${n.id}`,
      summary: `Notification "${subject}" delivered to ${target.username}`,
      data: { notificationId: n.id, to: target.username, type },
    });
  }
  return n;
}

// ---------------------------------------------------------------- failure guards

function signature(body) {
  if (!body || typeof body !== 'object') return 'no-body';
  const keys = Object.keys(body).filter((k) => k !== 'idempotencyKey' && k !== 'idempotency_key').sort();
  const parts = keys.map((k) => `${k}=${String(body[k]).slice(0, 40)}`);
  return parts.join('|').slice(0, 160) || 'empty';
}
function checkDuplicate(ctx, opKey, naturalKey) {
  const m = ctx.state.meta.idempotency;
  const provided = ctx.body && (ctx.body.idempotencyKey || ctx.body.idempotency_key);
  const sig = `${opKey}:${provided || naturalKey || signature(ctx.body)}`;
  if (m[sig]) {
    throw new AppError(409, 'duplicate_event',
      `Rejected as duplicate event: signature "${sig}" was first seen ${m[sig].at}.`,
      { key: sig, firstSeenAt: m[sig].at });
  }
  if (ctx.failure === 'duplicate_event') {
    m[sig] = { at: new Date().toISOString(), op: opKey, simulated: true };
    throw new AppError(409, 'duplicate_event',
      `Rejected as duplicate event (simulated): request signature "${sig}" recorded and rejected.`,
      { key: sig, simulated: true });
  }
  if (provided) m[sig] = { at: new Date().toISOString(), op: opKey };
}
function guardMutation(ctx, route) {
  if (route.method !== 'GET' && route.method !== 'HEAD') {
    if (ctx.failure === 'service_unavailable') {
      throw new AppError(503, 'service_unavailable',
        `The "${ctx.app.config.serviceName}" service is unavailable (simulated). State-changing requests are rejected; read requests still work.`,
        { service: ctx.app.config.serviceName, simulated: true });
    }
    const opKey = route.opKey || `${route.method} ${route.pattern}`;
    checkDuplicate(ctx, opKey, route.naturalKey ? route.naturalKey(ctx) : null);
  }
  if (route.auth === false) return;
  requireUser(ctx);
  if (route.perm) requirePermission(ctx, route.perm);
}

// ---------------------------------------------------------------- http helpers

function sendJSON(res, status, obj) {
  const body = JSON.stringify(obj, null, 2);
  res.writeHead(status, {
    'Content-Type': 'application/json; charset=utf-8',
    'Content-Length': Buffer.byteLength(body),
    'Cache-Control': 'no-store',
  });
  res.end(body);
}
function sendHTML(res, status, html) {
  res.writeHead(status, {
    'Content-Type': 'text/html; charset=utf-8',
    'Content-Length': Buffer.byteLength(html),
    'Cache-Control': 'no-store',
  });
  res.end(html);
}
function redirect(res, location) {
  res.writeHead(303, { Location: location });
  res.end();
}
function parseCookies(req) {
  const out = {};
  const raw = req.headers.cookie;
  if (!raw) return out;
  for (const part of raw.split(';')) {
    const i = part.indexOf('=');
    if (i > 0) out[part.slice(0, i).trim()] = decodeURIComponent(part.slice(i + 1).trim());
  }
  return out;
}
function readBody(req) {
  return new Promise((resolve, reject) => {
    const chunks = [];
    let size = 0;
    req.on('data', (c) => {
      size += c.length;
      if (size > MAX_BODY_BYTES) { reject(new AppError(413, 'payload_too_large', 'Request body exceeds 1 MB.')); req.destroy(); return; }
      chunks.push(c);
    });
    req.on('end', () => {
      const raw = Buffer.concat(chunks).toString('utf8');
      const ctype = String(req.headers['content-type'] || '');
      let body = {};
      if (raw) {
        if (ctype.includes('application/json')) {
          try { body = JSON.parse(raw); } catch { reject(new AppError(400, 'invalid_json', 'Request body is not valid JSON.')); return; }
        } else {
          const params = new URLSearchParams(raw);
          for (const [k, v] of params) {
            if (body[k] === undefined) body[k] = v;
            else if (Array.isArray(body[k])) body[k].push(v);
            else body[k] = [body[k], v];
          }
        }
      }
      resolve(body);
    });
    req.on('error', reject);
  });
}
function asList(v) {
  if (v === undefined || v === null || v === '') return [];
  if (Array.isArray(v)) return v.map(String);
  return String(v).split(',').map((s) => s.trim()).filter(Boolean);
}
function wantsHtml(req, url) {
  const fmt = url.searchParams.get('format');
  if (fmt === 'html') return true;
  if (fmt === 'json') return false;
  return String(req.headers.accept || '').includes('text/html');
}

// ---------------------------------------------------------------- routing

function matchRoute(route, method, pathname) {
  if (route.method !== method) return null;
  const rp = route.pattern.split('/').filter(Boolean);
  const ap = pathname.split('/').filter(Boolean);
  if (rp.length !== ap.length) return null;
  const params = {};
  for (let i = 0; i < rp.length; i++) {
    if (rp[i].startsWith(':')) params[rp[i].slice(1)] = decodeURIComponent(ap[i]);
    else if (rp[i] !== ap[i]) return null;
  }
  return params;
}

// ---------------------------------------------------------------- app

function createApp(config) {
  const required = ['family', 'appName', 'appTitle', 'port', 'rootDir', 'seedFn', 'routes', 'failures', 'serviceName'];
  for (const k of required) if (config[k] === undefined) throw new Error(`createApp: missing config.${k}`);

  const store = createStore(config.rootDir, config.seedFn);
  const startedAt = Date.now();
  const web = require('./web');

  const app = {
    config, store, startedAt,
    cookieName: `${config.appName}_session`,
  };

  function userFor(req, url) {
    const state = store.state;
    const token =
      (url.searchParams.get('token')) ||
      (req.headers.authorization && String(req.headers.authorization).replace(/^Bearer\s+/i, '')) ||
      parseCookies(req)[app.cookieName] || null;
    const u = token ? findUserByToken(state, token) : null;
    return u ? publicUser(u) : null;
  }
  function failureFor(req, url) {
    let name = url.searchParams.get('failure') || req.headers['x-failure-switch'] || null;
    if (name === null || name === '') return null;
    name = String(name).trim();
    const canon = FAILURE_ALIASES[name] || name;
    if (!FAILURE_SWITCHES.includes(canon)) {
      throw new AppError(400, 'unknown_failure_switch',
        `Unknown failure switch "${name}". Known switches: ${FAILURE_SWITCHES.join(', ')}.`,
        { known_switches: FAILURE_SWITCHES });
    }
    return canon;
  }
  function addFlash(base, ok, warn, err) {
    const u = new URL(base, 'http://localhost');
    ['ok', 'warn', 'err'].forEach((k) => { if (u.searchParams.has(k)) u.searchParams.delete(k); });
    if (ok) u.searchParams.set('ok', ok);
    if (warn) u.searchParams.set('warn', warn);
    if (err) u.searchParams.set('err', err);
    return u.pathname + (u.search || '');
  }
  function resetWorld() {
    const s = store.reset();
    s.meta.resets = (s.meta.resets || 0) + 1;
    pushEvent(s, null, 'world.reset', {
      subject: 'state', summary: `Demo world reset to seed state (reset #${s.meta.resets})`, data: { family: config.family },
    });
    store.save();
    return s;
  }
  function eventsJSON(limitRaw, since, type) {
    const limit = Math.min(parseInt(limitRaw || '100', 10) || 100, MAX_EVENTS);
    let list = store.state.events.slice();
    if (type) list = list.filter((e) => e.type === type);
    if (since) {
      const idx = list.findIndex((e) => e.id === since);
      list = idx >= 0 ? list.slice(idx + 1) : [];
    }
    return { ok: true, family: config.family, count: Math.min(list.length, limit), events: list.slice(-limit).reverse() };
  }

  const standardRoutes = [
    { kind: 'api', method: 'GET', pattern: '/healthz', auth: false, op: () => ({
      ok: true, family: config.family, app: config.appTitle, port: config.port,
      version: '1.0.0', uptimeSec: Math.round((Date.now() - startedAt) / 1000),
      events: store.state.events.length, resets: store.state.meta.resets,
    }) },
    { kind: 'api', method: 'GET', pattern: '/api/whoami', op: (ctx) => ({ ok: true, user: ctx.user }) },
    { kind: 'api', method: 'POST', pattern: '/api/login', auth: false, op: (ctx) => {
      const r = checkLogin(store.state, ctx.body.username, ctx.body.password);
      pushEvent(store.state, r.user.username, 'auth.login', {
        subject: `user:${r.user.id}`, summary: `${r.user.username} signed in`, data: { role: r.user.role },
      });
      return { ok: true, token: r.token, user: r.user, note: 'Send this token as "Authorization: Bearer <token>" or "?token=".' };
    } },
    { kind: 'api', method: 'GET', pattern: '/api/demo-hints', auth: false, op: () => ({
      ok: true,
      note: 'Fake demo world: credentials are assembled at runtime from fragments; no real credentials exist in source.',
      users: demoHints(store.state),
    }) },
    { kind: 'api', method: 'GET', pattern: '/api/events', auth: false, op: (ctx) => eventsJSON(ctx.query.get('limit'), ctx.query.get('since'), ctx.query.get('type')) },
    { kind: 'api', method: 'GET', pattern: '/api/failures', auth: false, op: () => ({
      ok: true,
      usage: 'Append ?failure=<switch> to any request, or send header "X-Failure-Switch: <switch>".',
      switches: config.failures,
    }) },
    { kind: 'api', method: 'POST', pattern: '/api/reset', auth: false, op: () => {
      const s = resetWorld();
      return { ok: true, message: 'Seed state restored.', resets: s.meta.resets, events: s.events.length };
    } },
    { kind: 'page', method: 'GET', pattern: '/login', auth: false, handler: (ctx) => web.loginPage(ctx, config, demoHints(store.state)) },
    { kind: 'page', method: 'GET', pattern: '/events', auth: false, handler: (ctx) => {
      const data = eventsJSON('200', null, null);
      return web.eventsPage(ctx, config, data.events);
    } },
    { kind: 'page', method: 'GET', pattern: '/failures', auth: false, handler: (ctx) => web.failuresPage(ctx, config) },
    { kind: 'form', method: 'POST', pattern: '/login', auth: false, op: (ctx) => {
      const r = checkLogin(store.state, ctx.body.username, ctx.body.password);
      pushEvent(store.state, r.user.username, 'auth.login', {
        subject: `user:${r.user.id}`, summary: `${r.user.username} signed in`, data: { role: r.user.role },
      });
      ctx.res.setHeader('Set-Cookie', `${app.cookieName}=${r.token}; Path=/; HttpOnly; SameSite=Lax`);
      const next = String(ctx.body.next || '/');
      return { redirect: next.startsWith('/') ? next : '/', message: `Signed in as ${r.user.name} (${r.user.role}).` };
    }, back: '/login' },
    { kind: 'form', method: 'POST', pattern: '/logout', auth: false, op: (ctx) => {
      ctx.res.setHeader('Set-Cookie', `${app.cookieName}=; Path=/; HttpOnly; Max-Age=0`);
      return { redirect: '/', message: 'Signed out.' };
    }, back: '/' },
    { kind: 'form', method: 'POST', pattern: '/reset', auth: false, op: () => {
      const s = resetWorld();
      ctx.res.setHeader('Set-Cookie', `${app.cookieName}=; Path=/; HttpOnly; Max-Age=0`);
      return { redirect: '/', message: `Demo world reset to seed state (reset #${s.meta.resets}). You were signed out.` };
    }, back: '/' },
  ];
  const routes = standardRoutes.concat(config.routes);

  async function handle(req, res) {
    const url = new URL(req.url, `http://localhost:${config.port}`);
    const method = req.method === 'HEAD' ? 'GET' : req.method;
    const pathname = url.pathname.length > 1 && url.pathname.endsWith('/') ? url.pathname.slice(0, -1) : url.pathname;
    try {
      const failure = failureFor(req, url);
      const user = userFor(req, url);
      let matched = null, params = null;
      for (const r of routes) {
        params = matchRoute(r, method, pathname);
        if (params) { matched = r; break; }
      }
      if (!matched) throw new AppError(404, 'not_found', `No route for ${method} ${pathname}.`);
      const body = (method === 'GET') ? {} : await readBody(req);
      const ctx = {
        app, req, res, url, params, query: url.searchParams, body, user, failure, store,
        state: store.state, save: () => store.save(),
        emit: (t, f) => emit(ctx, t, f),
        notify: (uid, t, s, b) => notify(ctx, uid, t, s, b),
      };
      guardMutation(ctx, matched);

      if (matched.kind === 'page') {
        const html = await matched.handler(ctx);
        sendHTML(res, 200, html);
        return;
      }
      if (matched.kind === 'api') {
        const result = await matched.op(ctx);
        store.save();
        sendJSON(res, 200, Object.assign({ ok: true }, result));
        return;
      }
      const result = await matched.op(ctx);
      store.save();
      const target = (result && result.redirect) || matched.back || '/';
      redirect(res, addFlash(target, result && result.message, result && result.warning, null));
    } catch (e) {
      const err = e instanceof AppError ? e : new AppError(500, 'internal_error', String((e && e.message) || e));
      if (!(e instanceof AppError)) console.error('[runtime] internal error:', e);
      try { store.save(); } catch { /* ignore */ }
      const isApi = pathname.startsWith('/api') || pathname === '/healthz';
      if (err.code === 'unknown_failure_switch' || isApi) {
        sendJSON(res, err.status, err.toJSON());
      } else if (method !== 'GET') {
        let back = '/';
        try {
          if (req.headers.referer) {
            const ref = new URL(req.headers.referer);
            if (ref.pathname.startsWith('/')) back = ref.pathname + (ref.search || '');
          }
        } catch { /* ignore */ }
        redirect(res, addFlash(back, null, null, `${err.message} [${err.code}]`));
      } else {
        sendHTML(res, err.status, web.errorPage(err, config));
      }
    }
  }

  function start() {
    store.state; // touch -> seeds if missing
    const server = http.createServer((req, res) => {
      handle(req, res).catch((e) => {
        try { sendJSON(res, 500, { ok: false, error: 'internal_error', message: String(e) }); } catch { /* ignore */ }
      });
    });
    server.listen(config.port, '127.0.0.1', () => {
      console.log(`[vwo-002 fixture] ${config.appTitle} (${config.family}) listening on http://localhost:${config.port}/`);
    });
    return server;
  }
  return { start, store, config, routes, resetWorld };
}

module.exports = {
  createApp, AppError, FAILURE_SWITCHES,
  emit, notify, requireUser, requirePermission, asList, publicUser, findUserById,
  pushEvent, nextId, signature, demoHints,
};
