#!/usr/bin/env node
'use strict';
/**
 * ForgeExport — purpose-built deployment-log export fixture (VWO-013 NST-B010,
 * §5 note 4 "download surface (as NST-B005)"). Serves ForgeOps' deployment log
 * as a downloadable text file from the browser.
 */
const http = require('http');
const PORT = parseInt(process.argv[2] || '4210', 10);
const FORGE = 'http://localhost:4102';

function fetchJSON(path) {
  return new Promise((resolve, reject) => {
    http.get(FORGE + path, (res) => {
      let b = ''; res.on('data', (c) => (b += c)); res.on('end', () => { try { resolve(JSON.parse(b)); } catch (e) { reject(e); } });
    }).on('error', reject);
  });
}

const PAGE = `<!doctype html><html><head><meta charset="utf-8"><title>ForgeExport — deployment log</title>
<style>body{font-family:system-ui,sans-serif;margin:0;background:#f4f6f8;color:#20262e}
.bar{background:#1f3a5f;color:#fff;padding:.5rem .9rem}h1{font-size:1rem;margin:0}
main{max-width:40rem;margin:1.2rem auto;padding:0 1rem}
.card{background:#fff;border:1px solid #d7dde3;border-radius:10px;padding:1rem}
.btn{display:inline-block;background:#1f3a5f;color:#fff;padding:.5rem 1rem;border-radius:7px;text-decoration:none}</style></head>
<body><div class="bar"><h1>ForgeExport — deployment log exports</h1></div>
<main><div class="card"><h2>This week's deployment log — ForgeOps</h2>
<p>Full deployment records (repo, environment, status, timestamps) as a text export.</p>
<p><a class="btn" id="dl" href="/export/deployments" download="forge-deployment-log.txt">Download deployment log</a></p>
</div></main></body></html>`;

http.createServer(async (req, res) => {
  const u = new URL(req.url, 'http://localhost');
  if (req.method === 'GET' && u.pathname === '/') {
    res.writeHead(200, { 'Content-Type': 'text/html; charset=utf-8' });
    return res.end(PAGE);
  }
  if (req.method === 'GET' && u.pathname === '/export/deployments') {
    try {
      const d = await fetchJSON('/api/deployments');
      const lines = ['# ForgeOps deployment log export (purpose-built, VWO-013 §5 note 4)',
        `# exported: ${new Date().toISOString()}`, ''];
      for (const dep of d.deployments) {
        lines.push(`${dep.id} repo=${dep.repoId} env=${dep.env} status=${dep.status} at=${dep.startedAt || dep.at || 'seed'}`);
      }
      lines.push('');
      res.writeHead(200, { 'Content-Type': 'text/plain; charset=utf-8', 'Content-Disposition': 'attachment; filename="forge-deployment-log.txt"' });
      return res.end(lines.join('\n'));
    } catch (e) { res.writeHead(502); return res.end('export failed: ' + e.message); }
  }
  res.writeHead(404); res.end('not found');
}).listen(PORT, '127.0.0.1', () => console.log('ForgeExport on http://127.0.0.1:' + PORT + '/'));
