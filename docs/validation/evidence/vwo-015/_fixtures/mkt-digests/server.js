#!/usr/bin/env node
'use strict';
/**
 * MarketplaceDigests — purpose-built digest-download fixture (VWO-013 NST-B005,
 * §5 note 4: "a browser-reachable download surface … FlowMart listing digests
 * as content"). Serves the current package digest file for a listing, with the
 * file's sha256 advertised on the page for the integrity check.
 * Zero dependencies. 127.0.0.1 only.
 */
const http = require('http');
const crypto = require('crypto');

const PORT = parseInt(process.argv[2] || '4209', 10);
const FLOWMART = 'http://localhost:4105';

function fetchJSON(path) {
  return new Promise((resolve, reject) => {
    http.get(FLOWMART + path, (res) => {
      let b = ''; res.on('data', (c) => (b += c)); res.on('end', () => { try { resolve(JSON.parse(b)); } catch (e) { reject(e); } });
    }).on('error', reject);
  });
}

async function buildDigest(slug) {
  const d = await fetchJSON('/api/packages/' + slug);
  const p = d.package || d;
  const versions = (p.versions || []).slice().sort((a, b) => (b.version > a.version ? 1 : -1));
  const current = versions[0];
  const lines = [
    `# FlowMart package digest — ${p.name} (${p.id})`,
    `current-version: ${current.version}`,
    `package-sha256: ${current.digest}`,
    `generated-by: MarketplaceDigests 1.0 (purpose-built, VWO-013 §5 note 4)`,
    '',
  ];
  const body = lines.join('\n');
  return { body, sha: crypto.createHash('sha256').update(body).digest('hex'), size: Buffer.byteLength(body), version: current.version, pkgSha: current.digest };
}

const PAGE = (slug, info) => `<!doctype html><html><head><meta charset="utf-8">
<title>MarketplaceDigests — ${slug}</title>
<style>body{font-family:system-ui,sans-serif;margin:0;background:#f6f7fb;color:#1e2430}
.bar{background:#3b3f8f;color:#fff;padding:.5rem .9rem}h1{font-size:1rem;margin:0}
main{max-width:42rem;margin:1.2rem auto;padding:0 1rem}
.card{background:#fff;border:1px solid #d9dcea;border-radius:10px;padding:1rem}
.btn{display:inline-block;background:#3b3f8f;color:#fff;padding:.5rem 1rem;border-radius:7px;text-decoration:none}
table{font-size:.85rem;border-collapse:collapse;margin-top:.6rem}td{padding:.25rem .5rem;border-bottom:1px solid #e6e8f2}</style></head>
<body><div class="bar"><h1>MarketplaceDigests — package digest exports</h1></div>
<main><div class="card">
<h2>Current digest file — ${slug}</h2>
<p><a class="btn" id="dl" href="/digest/${slug}" download="${slug}-digest.txt">Download digest file</a></p>
<table>
<tr><td>current version</td><td><b>${info.version}</b></td></tr>
<tr><td>file size</td><td><b>${info.size} bytes</b></td></tr>
<tr><td>file sha256</td><td><b>${info.sha}</b></td></tr>
<tr><td>package sha256 (content)</td><td>${info.pkgSha}</td></tr>
</table>
</div></main></body></html>`;

http.createServer(async (req, res) => {
  const u = new URL(req.url, 'http://localhost');
  const slug = (u.pathname.split('/')[2] || '').replace(/[^a-z0-9-]/g, '');
  if (req.method === 'GET' && u.pathname.startsWith('/digest/') && slug) {
    try {
      const { body } = await buildDigest(slug);
      res.writeHead(200, { 'Content-Type': 'text/plain; charset=utf-8', 'Content-Disposition': `attachment; filename="${slug}-digest.txt"` });
      return res.end(body);
    } catch (e) { res.writeHead(502); return res.end('digest build failed'); }
  }
  if (req.method === 'GET' && (u.pathname === '/' || u.pathname.startsWith('/listing/'))) {
    try {
      const s = slug || 'support-triage-assistant';
      const info = await buildDigest(s);
      res.writeHead(200, { 'Content-Type': 'text/html; charset=utf-8' });
      return res.end(PAGE(s, info));
    } catch (e) { res.writeHead(502); return res.end(e.message); }
  }
  res.writeHead(404); res.end('not found');
}).listen(PORT, '127.0.0.1', () => console.log('MarketplaceDigests on http://127.0.0.1:' + PORT + '/'));
