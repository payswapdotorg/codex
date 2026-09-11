#!/usr/bin/env node
'use strict';
/**
 * ForgeOps Deploy Console — DESKTOP-TARGET TERMINAL APPLICATION (VWO-002).
 *
 * This is the honest "desktop-capable application surface" for the software family:
 * an interactive, xterm-able command-line client that talks to the ForgeOps HTTP API.
 * It works in any terminal (readline) and also accepts piped stdin for scripted runs:
 *
 *     node deploy-console.js [--port 4102]
 *     printf 'login raj.patel <password>\ndeployments\npromote dep-1101\nquit\n' | node deploy-console.js
 *
 * It is a CLIENT only: all state lives in the fixture server. This is not an E2B
 * desktop environment — VWO-001 owns that environment question.
 */

const readline = require('node:readline');

const portArg = process.argv.includes('--port') ? process.argv[process.argv.indexOf('--port') + 1] : null;
const PORT = parseInt(portArg || process.env.FIXTURE_PORT || '4102', 10);
const BASE = `http://localhost:${PORT}`;

let token = null;
let who = null;

async function api(method, path, body) {
  const headers = { 'content-type': 'application/json' };
  if (token) headers.authorization = `Bearer ${token}`;
  const res = await fetch(`${BASE}${path}`, {
    method, headers,
    body: body === undefined ? undefined : JSON.stringify(body),
  });
  const data = await res.json().catch(() => ({}));
  return { status: res.status, data };
}

const HELP = `
Commands:
  help                          show this help
  login <username> <password>   sign in (demo passwords: GET /api/demo-hints or the login page)
  whoami                        show the signed-in user
  repos                         list repositories
  deployments [repo]            list deployments (optionally filtered by repo id)
  promote <deploymentId>        promote a successful staging deployment to production
  rollback <deploymentId>       roll back a production deployment to the prior version
  incidents                     list incidents
  events [limit]                show recent events
  switch <name>                 arm a failure switch for the NEXT command (e.g. switch data_conflict)
  quit                          exit
Notes: all commands go through the real HTTP API (no direct state access).`;

let nextFailure = null;

function fmtDep(d) {
  return `  ${d.id}  ${d.env.padEnd(10)} ${String(d.version).padEnd(9)} ${d.status.padEnd(10)} artifact=${d.artifactId} repo=${d.repoId}`;
}
function fmtInc(i) {
  return `  ${i.id}  ${i.severity.padEnd(4)} ${String(i.status).padEnd(10)} ${i.title} (service ${i.serviceId})`;
}

async function command(line) {
  const parts = line.trim().split(/\s+/);
  const cmd = (parts[0] || '').toLowerCase();
  if (!cmd) return;
  try {
    if (cmd === 'quit' || cmd === 'exit') { process.exit(0); return; }
    if (cmd === 'help' || cmd === '?') { console.log(HELP); return; }
    if (cmd === 'login') {
      const [, username, password] = parts;
      if (!username || !password) { console.log('usage: login <username> <password>'); return; }
      const r = await api('POST', '/api/login', { username, password });
      if (r.status !== 200) { console.log(`login failed: ${r.data.message || r.status}`); return; }
      token = r.data.token; who = r.data.user;
      console.log(`signed in as ${who.name} (${who.role}). token stored for this session.`);
      return;
    }
    if (cmd === 'whoami') { console.log(who ? `${who.name} (${who.role}) permissions: ${who.permissions.join(', ')}` : 'not signed in'); return; }
    if (cmd === 'switch') {
      nextFailure = parts[1] || null;
      console.log(nextFailure ? `failure switch armed for next command: ${nextFailure}` : 'failure switch disarmed.');
      return;
    }
    const q = nextFailure ? `?failure=${nextFailure}` : '';
    nextFailure = null;
    if (cmd === 'repos') {
      const r = await api('GET', '/api/repos');
      for (const repo of r.data.repos || []) console.log(`  ${repo.id}  ${repo.name.padEnd(18)} ${repo.language.padEnd(11)} staging=${repo.currentStagingVersion} prod=${repo.productionVersion}`);
      return;
    }
    if (cmd === 'deployments') {
      const r = await api('GET', `/api/deployments${q}`);
      const deps = (r.data.deployments || []).filter((d) => !parts[1] || d.repoId === parts[1]);
      console.log(`deployments (${deps.length}):`);
      for (const d of deps) console.log(fmtDep(d));
      return;
    }
    if (cmd === 'promote' || cmd === 'rollback') {
      if (!parts[1]) { console.log(`usage: ${cmd} <deploymentId>`); return; }
      const r = await api('POST', `/api/deployments/${cmd}${q}`, { deploymentId: parts[1] });
      if (r.status === 200) console.log(`OK: ${r.data.message}`);
      else console.log(`FAILED ${r.status}: ${r.data.error} — ${r.data.message || ''}`);
      return;
    }
    if (cmd === 'incidents') {
      const r = await api('GET', `/api/incidents${q}`);
      console.log(`incidents (${(r.data.incidents || []).length}):`);
      for (const i of r.data.incidents || []) console.log(fmtInc(i));
      return;
    }
    if (cmd === 'events') {
      const r = await api('GET', `/api/events?limit=${parts[1] || 10}${q ? '&' + q.slice(1) : ''}`);
      for (const e of (r.data.events || []).slice(0, parseInt(parts[1] || '10', 10))) console.log(`  ${e.id} ${e.type.padEnd(24)} ${e.summary}`);
      return;
    }
    console.log(`unknown command "${cmd}" — type "help".`);
  } catch (e) {
    console.log(`error: ${e.message} (is the server running on ${BASE}?)`);
  }
}

async function main() {
  console.log(`ForgeOps Deploy Console (desktop-target terminal application)`);
  console.log(`server: ${BASE} — start it with: node server.js --port ${PORT}`);
  console.log(`type "help" for commands.\n`);
  const isTTY = Boolean(process.stdin.isTTY);
  const rl = readline.createInterface({ input: process.stdin, output: process.stdout, prompt: 'forgeops> ' });
  // Serialize commands so piped scripts execute in order and finish before exit.
  let queue = Promise.resolve();
  rl.on('line', (line) => {
    queue = queue
      .then(() => command(line))
      .catch((e) => console.log(`error: ${e.message}`))
      .then(() => { if (isTTY) rl.prompt(); });
  });
  rl.on('close', () => { queue = queue.then(() => process.exit(0)); });
  if (isTTY) rl.prompt();
}

main();
