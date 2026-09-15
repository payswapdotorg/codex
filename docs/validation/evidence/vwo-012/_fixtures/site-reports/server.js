#!/usr/bin/env node
'use strict';
/**
 * SiteReports — purpose-built report-export fixture (VWO-012, §5 note 4).
 * A plausible small "export the project report" surface: pulls SiteBuild's
 * project data server-side and serves it as a downloadable report file
 * (the catalog's "download surface (purpose-built fixture endpoint serving
 * a report file)" for NST-D005). Zero dependencies; 127.0.0.1 only.
 */
const http = require('http');

const PORT = parseInt(process.argv[2] || '4205', 10);
const SITEBUILD = 'http://localhost:4101';

function fetchJSON(path) {
  return new Promise((resolve, reject) => {
    http.get(SITEBUILD + path, (res) => {
      let b = '';
      res.on('data', (c) => (b += c));
      res.on('end', () => { try { resolve(JSON.parse(b)); } catch (e) { reject(e); } });
    }).on('error', reject);
  });
}

const PAGE = `<!doctype html><html><head><meta charset="utf-8">
<title>SiteReports — Project Report Exports</title>
<style>
 body{font-family:system-ui,sans-serif;margin:0;background:#f4f6f8;color:#20262e}
 .bar{background:#274156;color:#fff;padding:.5rem .9rem}
 h1{font-size:1rem;margin:0;font-weight:600}
 main{max-width:46rem;margin:1.2rem auto;padding:0 1rem}
 .card{background:#fff;border:1px solid #d7dde3;border-radius:10px;padding:1rem;margin-bottom:1rem}
 .card h2{font-size:.95rem;margin:.1rem 0 .6rem}
 .btn{display:inline-block;background:#274156;color:#fff;border:none;font:inherit;padding:.5rem 1rem;border-radius:7px;cursor:pointer;text-decoration:none}
 .btn:hover{background:#33526e}
 .muted{color:#5a6472;font-size:.82rem}
</style></head><body>
<div class="bar"><h1>SiteReports — weekly report exports</h1></div>
<main>
 <div class="card">
  <h2>Harborview Tower — weekly project report</h2>
  <p class="muted">Compiled from SiteBuild live project data (progress, reports, tasks). Download and file it in the weekly folder.</p>
  <a class="btn" id="dl" href="/report/harborview-tower" download="harborview-tower-report.json">Download report</a>
 </div>
</main></body></html>`;

http.createServer(async (req, res) => {
  const u = new URL(req.url, 'http://localhost');
  if (req.method === 'GET' && u.pathname === '/') {
    res.writeHead(200, { 'Content-Type': 'text/html; charset=utf-8' });
    return res.end(PAGE);
  }
  if (req.method === 'GET' && u.pathname === '/report/harborview-tower') {
    try {
      const d = await fetchJSON('/api/projects/p-101');
      const report = {
        generatedBy: 'SiteReports 1.0 (purpose-built export surface, VWO-012 §5 note 4)',
        project: d.project,
        reports: d.reports,
        tasks: d.tasks,
        exportedAt: new Date().toISOString(),
      };
      const body = JSON.stringify(report, null, 2);
      res.writeHead(200, {
        'Content-Type': 'application/json',
        'Content-Disposition': 'attachment; filename="harborview-tower-report.json"',
      });
      return res.end(body);
    } catch (e) {
      res.writeHead(502, { 'Content-Type': 'text/plain' });
      return res.end('report generation failed: ' + e.message);
    }
  }
  res.writeHead(404); res.end('not found');
}).listen(PORT, '127.0.0.1', () => console.log('SiteReports on http://127.0.0.1:' + PORT + '/'));
