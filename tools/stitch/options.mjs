#!/usr/bin/env node
// design-options: four Google Stitch mockups of one screen, written as a UI
// survey (_shots/uipass/survey-design-<tag>/ + index.md) so /carbon-ui
// design-<tag> critiques them unchanged. Run by design-options.yml from the
// repo root; never by Claude Code or Cowork (the call is a workflow step).
//
//   node tools/stitch/options.mjs [--dry-run] [--body-file F | "<command>"] [--out F]
//
// <command> is the comment's first line, "/design-options <tag> <screen>
// [--app <name>] -- <brief>"; lines after it continue the brief.
// --dry-run: parse, build the prompts and the file plan, print them, exit 0.
// No Stitch call, no dependencies loaded, no key needed.
// --out F: a JSON result for the workflow ({status, ...}); status is ok,
// usage, refused (HTTP 401/403 from Stitch), no-tool, shape (a response the
// script cannot read, including a non-image where an image was expected; the
// first 2 KB of such a body go to the job summary, values redacted) or error.
// Exit 0 only for ok. The run log carries the tool list, each call's result
// content types and shape, and each image's source and first bytes, never values.
// Image path per variant, logged as "via": an image block from the generator or
// get_screen; else download_file on the screenshot's resource name (its image
// block); else the screenshot downloadUrl, either a JSON wrapper whose base64
// field is decoded (url-json) or raw image bytes (url-bytes).
//
// Env: STITCH_API_KEY (or BTS_STITCH_KEY_OVERRIDE, honoured only when set),
// VARS_JSON (toJSON(vars): BTS_STITCH_PROJECT[_<APP>]), UI_KIND, REPO,
// GITHUB_STEP_SUMMARY. The key goes into one request header and nowhere else.
// From gk1016/bts-template core/tools/stitch (frago plan 2026-11-15-design-options-lane).

import { createHash } from 'node:crypto';
import { existsSync, mkdirSync, readFileSync, rmSync, appendFileSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';

const ENDPOINT = 'https://stitch.googleapis.com/mcp';
const MAX_BYTES = 400 * 1024;
const QUALITIES = [85, 75, 65, 55]; // shots.ps1 behaviour
const CALL_TIMEOUT_MS = 15 * 60 * 1000;
const HINTS = [
  ['layout-first', 'try a distinctly different arrangement of the main regions; keep everything the brief asks for'],
  ['density-first', 'fit the most information per screen that stays legible at a glance'],
  ['hierarchy-first', 'make the single most important reading unmistakable, everything else clearly secondary'],
  ['motion-and-depth-first', 'separate what is live from what is fixed with layering, elevation and implied motion'],
];

class Stop extends Error {
  constructor(status, message, extra = {}) { super(message); this.status = status; this.extra = extra; }
}

// ---- arguments -------------------------------------------------------------
const argv = process.argv.slice(2);
let dry = false, bodyFile = '', outFile = '', bodyArg = '';
for (let i = 0; i < argv.length; i++) {
  const a = argv[i];
  if (a === '--dry-run') dry = true;
  else if (a === '--body-file') bodyFile = argv[++i] ?? '';
  else if (a === '--out') outFile = argv[++i] ?? '';
  else bodyArg = a;
}

function finish(result) {
  if (outFile) writeFileSync(outFile, JSON.stringify(result, null, 2) + '\n');
  process.exit(result.status === 'ok' ? 0 : 1);
}

function summary(text) {
  const f = process.env.GITHUB_STEP_SUMMARY;
  if (f) appendFileSync(f, text + '\n');
  else console.log(text);
}

const body = bodyFile ? readFileSync(bodyFile, 'utf8') : (bodyArg || process.env.BODY || '');

function parse(text) {
  const lines = text.replace(/\r/g, '').split('\n');
  const first = (lines[0] ?? '').trim();
  const m = /^\/design-options\s+([a-z0-9-]+)\s+(\S+)(?:\s+--app\s+([A-Za-z0-9_-]+))?\s+--(?:\s+(.*))?$/.exec(first);
  if (!m) return null;
  const brief = [m[4] ?? '', ...lines.slice(1)].join('\n').trim();
  if (!brief) return null;
  return { tag: m[1], screen: m[2], app: m[3] ?? '', brief };
}

const cmd = parse(body);
if (!cmd) {
  const first = body.replace(/\r/g, '').split('\n')[0] ?? '';
  console.error(`usage: /design-options <tag> <screen> [--app <name>] -- <brief>; got: ${first.slice(0, 200)}`);
  finish({ status: 'usage', message: `usage: /design-options <tag> <screen> [--app <name>] -- <brief>; got: ${first.slice(0, 200)}` });
}

// ---- prompt ----------------------------------------------------------------
const root = process.cwd();
const repo = process.env.REPO || '';
const repoName = repo.split('/').pop() || 'repo';
const uiKind = process.env.UI_KIND || 'an application';
const device = /\b(ipad|tablet)\b/i.test(uiKind) && !/\b(iphone|ios app|mobile)\b/i.test(uiKind) ? 'TABLET'
  : /\b(ios|iphone|android|mobile)\b/i.test(uiKind) ? 'MOBILE' : 'DESKTOP';

function brand() {
  const doc = cmd.app ? join(cmd.app, 'docs', 'design-language.md') : join('docs', 'design-language.md');
  if (existsSync(join(root, doc))) return { source: doc, text: readFileSync(join(root, doc), 'utf8').trim() };
  const review = join('claude', 'review', 'carbon-ui.md');
  if (existsSync(join(root, review))) {
    const m = /^## Brand rules[^\n]*\n([\s\S]*?)(?=^## |(?![\s\S]))/m.exec(readFileSync(join(root, review), 'utf8'));
    if (m && m[1].trim()) return { source: `${review} (Brand rules)`, text: m[1].trim() };
  }
  return { source: 'none', text: '' };
}

const b = brand();
const variants = HINTS.map(([key, hint], i) => {
  const prompt = [
    cmd.brief,
    '',
    `Screen: ${cmd.screen}. This is ${uiKind}.`,
    `Variant ${i + 1} of 4, ${key}: ${hint}.`,
    ...(b.text ? ['', 'Design language (follow it):', b.text] : []),
  ].join('\n');
  return {
    n: i + 1, key, hint, prompt,
    hash: createHash('sha256').update(prompt).digest('hex').slice(0, 12),
    file: `survey-design-${cmd.tag}-v${i + 1}.jpg`,
  };
});
const rel = `_shots/uipass/survey-design-${cmd.tag}`;
const dir = join(root, rel);
const vars = (() => { try { return JSON.parse(process.env.VARS_JSON || '{}'); } catch { return {}; } })();
const appKey = cmd.app.toUpperCase().replace(/[^A-Z0-9]/g, '_');
const projectVar = cmd.app ? `BTS_STITCH_PROJECT_${appKey}` : 'BTS_STITCH_PROJECT';
const projectTitle = cmd.app ? `${repoName} ${cmd.app}` : repoName;

if (dry) {
  console.log(`command: tag=${cmd.tag} screen=${cmd.screen} app=${cmd.app || '-'}`);
  console.log(`device: ${device} (from UI_KIND "${uiKind}")`);
  console.log(`brand: ${b.source}`);
  console.log(`project: ${vars[projectVar] ? `${projectVar}=${vars[projectVar]}` : `${projectVar} unset -> create_project "${projectTitle}"`}`);
  console.log(`branch: ui/survey-design-${cmd.tag} (force push, replaces)`);
  console.log(`----- prompt v1 -----\n${variants[0].prompt}\n----- end -----`);
  for (const v of variants) console.log(`${rel}/${v.file}  v${v.n} ${v.key}  prompt ${v.hash}`);
  console.log(`${rel}/index.md`);
  finish({ status: 'ok', dry: true, ...cmd, device, brand: b.source, files: variants.map(v => v.file) });
}

// ---- Stitch ----------------------------------------------------------------
// Tool names are not hard-coded beyond a preference order: the official server
// is listed first and its own input schemas decide the argument names.
function harvest(res) {
  const out = { images: [], urls: [], screens: [], projects: [], texts: [] };
  // key is the last property name; path is the dotted chain (screenshot.downloadUrl
  // vs htmlCode.downloadUrl), which is what tells an image URL from a page URL.
  const walk = (v, key = '', path = key) => {
    if (v == null) return;
    if (typeof v === 'string') {
      for (const m of v.matchAll(/projects\/([A-Za-z0-9_-]+)\/screens\/([A-Za-z0-9_-]+)/g)) { out.projects.push(m[1]); out.screens.push(m[2]); }
      for (const m of v.matchAll(/projects\/([A-Za-z0-9_-]+)/g)) out.projects.push(m[1]);
      if (/^https?:\/\//.test(v)) out.urls.push({ key: path, url: v });
      if (/^(screen_?id|screenid)$/i.test(key)) out.screens.push(v);
      if (/^(project_?id|projectid)$/i.test(key)) out.projects.push(v);
      return;
    }
    if (typeof v === 'number') {
      if (/^(screen_?id)$/i.test(key)) out.screens.push(String(v));
      if (/^(project_?id)$/i.test(key)) out.projects.push(String(v));
      return;
    }
    if (Array.isArray(v)) { v.forEach(x => walk(x, key, path)); return; }
    if (typeof v === 'object') for (const [k, x] of Object.entries(v)) walk(x, k, path ? `${path}.${k}` : k);
  };
  for (const c of res?.content ?? []) {
    if (c.type === 'image' && c.data) out.images.push(Buffer.from(c.data, 'base64'));
    else if (c.type === 'text' && typeof c.text === 'string') {
      out.texts.push(c.text);
      try { walk(JSON.parse(c.text)); } catch { walk(c.text); }
    } else if (c.type === 'resource' && c.resource) {
      if (c.resource.blob && /^image\//.test(c.resource.mimeType ?? '')) out.images.push(Buffer.from(c.resource.blob, 'base64'));
      walk(c.resource);
    } else walk(c);
  }
  walk(res?.structuredContent);
  return out;
}

// The shape of a response, never its values: what the stop-and-ask report needs.
function shape(v, depth = 0) {
  if (depth > 4) return '…';
  if (Array.isArray(v)) return v.length ? [shape(v[0], depth + 1)] : [];
  if (v && typeof v === 'object') return Object.fromEntries(Object.entries(v).slice(0, 20).map(([k, x]) => [k, shape(x, depth + 1)]));
  if (typeof v === 'string') return /^https?:\/\//.test(v) ? `url(${new URL(v).host})` : `string(${v.length})`;
  return typeof v;
}

// A tool result's content types and, for JSON text, its shape: logged per call.
function describe(res) {
  const parts = (res?.content ?? []).map(c => {
    if (c.type === 'image') return `image(${c.mimeType ?? 'no mimeType'}, base64 ${c.data?.length ?? 0})`;
    if (c.type === 'text') {
      let s = '';
      try { s = ' ' + JSON.stringify(shape(JSON.parse(c.text))); } catch { /* plain text */ }
      return `text(${c.text?.length ?? 0})${s}`;
    }
    if (c.type === 'resource') return `resource(${c.resource?.mimeType ?? 'no mimeType'}${c.resource?.blob ? `, base64 ${c.resource.blob.length}` : ''})`;
    return `${c.type}${c.mimeType ? `(${c.mimeType})` : ''}`;
  });
  if (res?.structuredContent) parts.push(`structuredContent ${JSON.stringify(shape(res.structuredContent))}`);
  return (parts.join(' | ') || 'no content').slice(0, 3000);
}

// Image URLs by path: screenshot/image first, never an HTML/code download.
function imageUrl(urls, fallback = /download/i) {
  const ok = urls.filter(x => !/html|code|css/i.test(x.key));
  return ok.find(x => /screenshot|image|thumbnail/i.test(x.key)) ?? ok.find(x => fallback.test(x.key));
}

function sniff(buf) {
  if (buf.length >= 4 && buf[0] === 0x89 && buf.toString('latin1', 1, 4) === 'PNG') return 'png';
  if (buf.length >= 3 && buf[0] === 0xff && buf[1] === 0xd8 && buf[2] === 0xff) return 'jpeg';
  if (buf.length >= 12 && buf.toString('latin1', 0, 4) === 'RIFF' && buf.toString('latin1', 8, 12) === 'WEBP') return 'webp';
  if (buf.length >= 3 && buf.toString('latin1', 0, 3) === 'GIF') return 'gif';
  return '';
}
const hex8 = buf => buf.subarray(0, 8).toString('hex');

// A JSON body carrying the image: a string field >= 10 KB (an "ima..." key
// first, then the longest) whose base64 decodes to image bytes. A data: URL
// prefix is stripped. Returns { key, buf } or null; the key is logged, never the value.
function jsonImage(text) {
  let doc;
  try { doc = JSON.parse(text); } catch { return null; }
  const cands = [];
  const walk = (v, key = '') => {
    if (typeof v === 'string') { if (v.length >= 10240) cands.push({ key, v }); }
    else if (Array.isArray(v)) v.forEach(x => walk(x, key));
    else if (v && typeof v === 'object') for (const [k, x] of Object.entries(v)) walk(x, k);
  };
  walk(doc);
  cands.sort((a, b) => (/^ima/i.test(b.key) - /^ima/i.test(a.key)) || b.v.length - a.v.length);
  for (const c of cands) {
    const buf = Buffer.from(c.v.replace(/^data:[^,]*,/, ''), 'base64');
    if (sniff(buf)) return { key: c.key, buf };
  }
  return null;
}

// get_screen's screenshot block ({ name, downloadUrl, mimeType }) from its JSON
// text or structuredContent; null when there is none.
function screenshotRef(res) {
  const docs = [res?.structuredContent, ...(res?.content ?? []).filter(c => c.type === 'text')
    .map(c => { try { return JSON.parse(c.text); } catch { return null; } })];
  let found = null;
  const walk = (v) => {
    if (found || !v || typeof v !== 'object') return;
    if (Array.isArray(v)) { v.forEach(walk); return; }
    if (v.screenshot && typeof v.screenshot === 'object') { found = v.screenshot; return; }
    Object.values(v).forEach(walk);
  };
  docs.forEach(walk);
  return found;
}

function pickEnum(schema, want) {
  const en = schema?.enum;
  if (!Array.isArray(en) || !en.length) return want;
  return en.find(e => String(e).toUpperCase().includes(want)) ?? en.find(e => /unspecified/i.test(String(e))) ?? en[0];
}

function fill(tool, want) {
  const props = tool.inputSchema?.properties ?? {};
  const required = tool.inputSchema?.required ?? [];
  const args = {};
  for (const [name, schema] of Object.entries(props)) {
    const n = name.toLowerCase();
    if (want.url != null && /url|uri/.test(n)) args[name] = want.url;
    else if (want.file != null && /^(name|file|file_?name|file_?id|resource|resource_?name)$/.test(n)) args[name] = want.file;
    else if (/screen/.test(n) && want.screen != null) args[name] = want.screen;
    else if (/project/.test(n) && want.project != null) args[name] = want.project;
    else if (/prompt|text|description|instruction/.test(n) && want.prompt != null) args[name] = want.prompt;
    else if (/title|display/.test(n) && want.title != null) args[name] = want.title;
    else if (n === 'name' && want.resource != null) args[name] = want.resource;
    else if (n === 'name' && want.title != null) args[name] = want.title;
    else if (/device|platform|form_?factor/.test(n) && want.device) args[name] = pickEnum(schema, want.device);
  }
  const missing = required.filter(r => !(r in args));
  if (missing.length) {
    throw new Stop('shape', `${tool.name}: required ${missing.join(', ')} not mappable`, { schema: tool.inputSchema });
  }
  return args;
}

function httpStatus(err) {
  if (typeof err?.code === 'number' && err.code >= 400 && err.code < 600) return err.code;
  const m = /\b(401|403)\b/.exec(String(err?.message ?? ''));
  return m ? Number(m[1]) : 0;
}

async function main() {
  const key = process.env.BTS_STITCH_KEY_OVERRIDE || process.env.STITCH_API_KEY;
  if (!key) throw new Stop('error', 'STITCH_API_KEY is empty (repo secret missing)');
  const { Client } = await import('@modelcontextprotocol/sdk/client/index.js');
  const { StreamableHTTPClientTransport } = await import('@modelcontextprotocol/sdk/client/streamableHttp.js');
  const sharp = (await import('sharp')).default;

  const transport = new StreamableHTTPClientTransport(new URL(ENDPOINT), {
    requestInit: { headers: { 'X-Goog-Api-Key': key } },
  });
  const client = new Client({ name: 'bts-design-options', version: '1.0.0' });
  const call = async (tool, args) => {
    let res;
    try {
      res = await client.callTool({ name: tool.name, arguments: args }, undefined, { timeout: CALL_TIMEOUT_MS, resetTimeoutOnProgress: true });
    } catch (err) {
      const s = httpStatus(err);
      if (s === 401 || s === 403) throw new Stop('refused', `Stitch refused (status ${s})`, { http: s, tool: tool.name });
      throw err;
    }
    if (res?.isError) {
      const text = (res.content ?? []).filter(c => c.type === 'text').map(c => c.text).join(' ').slice(0, 400);
      const s = httpStatus({ message: text });
      if (s === 401 || s === 403 || /PERMISSION_DENIED|UNAUTHENTICATED/.test(text)) {
        throw new Stop('refused', `Stitch refused (status ${s || 403})`, { http: s || 403, tool: tool.name });
      }
      throw new Stop('error', `${tool.name} returned an error: ${text}`);
    }
    console.log(`${tool.name} result: ${describe(res)}`);
    return res;
  };

  try {
    await client.connect(transport);
  } catch (err) {
    const s = httpStatus(err);
    if (s === 401 || s === 403) throw new Stop('refused', `Stitch refused (status ${s})`, { http: s });
    throw err;
  }
  const { tools } = await client.listTools();
  summary(`### Stitch tools (${ENDPOINT})\n\n| Tool | Required arguments |\n|---|---|`);
  for (const t of tools) summary(`| \`${t.name}\` | ${(t.inputSchema?.required ?? []).map(r => `\`${r}\``).join(', ') || '-'} |`);
  const names = tools.map(t => t.name);
  console.log(`stitch tools: ${names.join(', ')}`);
  const find = (...res) => res.map(r => tools.find(t => r.test(t.name))).find(Boolean);
  const tGen = find(/^generate_screen_from_text$/, /generate.*screen/i, /screen.*generat/i);
  const tCreate = find(/^create_project$/, /create.*project/i);
  const tImage = find(/^get_screen_image$/, /screen.*image|image.*screen/i, /^get_screen$/);
  const tDownload = find(/^download_file$/, /download/i);
  const tVariants = find(/variant/i);
  console.log(`download tool: ${tDownload ? tDownload.name : 'absent'}; variants tool: ${tVariants ? `${tVariants.name} present (not used by this rev)` : 'absent'}`);
  if (!tGen) throw new Stop('no-tool', 'no screen-generation tool on the Stitch server', { tools: names });
  if (tVariants) summary(`\nThe server offers \`${tVariants.name}\`; this run makes one \`${tGen.name}\` call per variant so each carries its own hint.`);

  // Project: the repo variable, else create one and report its id.
  let project = vars[projectVar] || '';
  let created = false;
  if (!project) {
    if (!tCreate) throw new Stop('no-tool', `${projectVar} is unset and the server has no create-project tool`, { tools: names });
    const h = harvest(await call(tCreate, fill(tCreate, { title: projectTitle })));
    project = h.projects[0] ?? '';
    if (!project) throw new Stop('shape', `${tCreate.name} returned no project id`, { shape: shape(h.texts.map(t => { try { return JSON.parse(t); } catch { return t; } })) });
    created = true;
  }
  summary(`\n**Stitch project:** \`${project}\`${created ? ` (created "${projectTitle}"; store it as repo variable \`${projectVar}\`)` : ` (\`${projectVar}\`)`}`);

  rmSync(dir, { recursive: true, force: true });
  mkdirSync(dir, { recursive: true });

  const fetchImage = async (url) => {
    const host = new URL(url).host;
    let r = await fetch(url, { redirect: 'follow' });
    // The key goes only to Google API hosts, never to an arbitrary URL.
    if ((r.status === 401 || r.status === 403) && host.endsWith('.googleapis.com')) {
      r = await fetch(url, { redirect: 'follow', headers: { 'X-Goog-Api-Key': key } });
    }
    if (!r.ok) throw new Stop('shape', `image fetch from ${host} returned HTTP ${r.status}`, { host, http: r.status });
    const ct = r.headers.get('content-type') ?? '';
    const buf = Buffer.from(await r.arrayBuffer());
    console.log(`  image url ${host}: HTTP ${r.status}, content-type ${ct || '-'}, ${buf.length} bytes, first 8 ${hex8(buf)}`);
    // Stitch answers the screenshot downloadUrl with a JSON wrapper around base64.
    if (/json/i.test(ct) || buf[0] === 0x7b) {
      const j = jsonImage(buf.toString('utf8'));
      if (j) {
        console.log(`  image url ${host}: JSON wrapper, base64 field "${j.key}" -> ${j.buf.length} bytes, first 8 ${hex8(j.buf)}`);
        return { buf: j.buf, how: `url-json (${host})` };
      }
    }
    // An image/* header, or (octet-stream from a bucket) bytes that are one.
    if (!/^image\//i.test(ct) && !sniff(buf)) {
      summary(`\n**Image URL from \`${host}\` returned \`${ct || 'no content-type'}\`, not an image.** First 2 KB of the body, values redacted:\n\n~~~~\n${redact(buf.subarray(0, 2048).toString('utf8'))}\n~~~~`);
      throw new Stop('shape', `image URL from ${host} returned ${ct || 'no content-type'}, not an image; first 2 KB (redacted) in the job summary`, { host, contentType: ct });
    }
    return { buf, how: `url-bytes (${host})` };
  };

  // download_file on the screenshot's resource name; its image block (or a base64
  // field in its JSON text). null on any non-refusal failure, so the URL path runs.
  const download = async (shot) => {
    let args;
    try {
      args = fill(tDownload, { project, file: shot.name, url: shot.downloadUrl });
    } catch (err) {
      if (!(err instanceof Stop)) throw err;
      console.log(`  ${tDownload.name}: ${err.message}; input properties ${Object.keys(tDownload.inputSchema?.properties ?? {}).join(', ') || '-'}; falling back to the URL`);
      return null;
    }
    let res;
    try {
      res = await call(tDownload, args);
    } catch (err) {
      if (err instanceof Stop && err.status === 'refused') throw err;
      console.log(`  ${tDownload.name} failed: ${String(err?.message ?? err).slice(0, 300)}; falling back to the URL`);
      return null;
    }
    const d = harvest(res);
    if (d.images[0]) return d.images[0];
    for (const t of d.texts) {
      const j = jsonImage(t);
      if (j) { console.log(`  ${tDownload.name}: base64 field "${j.key}" in its text result`); return j.buf; }
    }
    console.log(`  ${tDownload.name} gave no image content; falling back to the URL`);
    return null;
  };
  // Shape for JSON; otherwise quoted values and token-like runs blanked, the key never.
  const redact = (text) => {
    try { return JSON.stringify(shape(JSON.parse(text)), null, 1); } catch { /* not JSON */ }
    return text.split(key).join('[key]')
      .replace(/(["'])[^"'\n]*\1/g, '$1…$1')
      .replace(/[A-Za-z0-9_+/=-]{16,}/g, m => `[${m.length} chars]`)
      .replace(/~{3,}/g, '~~');
  };

  const results = [];
  for (const v of variants) {
    const gen = await call(tGen, fill(tGen, { project, prompt: v.prompt, device }));
    const g = harvest(gen);
    const screen = g.screens[0] ?? '';
    let img = g.images[0];
    let via = img ? tGen.name : '';
    if (!img && tImage && screen) {
      const sres = await call(tImage, fill(tImage, { project, screen, resource: `projects/${project}/screens/${screen}` }));
      const h = harvest(sres);
      img = h.images[0];
      via = tImage.name;
      const shot = img ? null : screenshotRef(sres);
      if (!img && tDownload && shot?.name) {
        img = await download(shot);
        if (img) via = tDownload.name;
      }
      if (!img) {
        const u = (shot?.downloadUrl ? { url: shot.downloadUrl } : null) ?? imageUrl(h.urls, /download|url/i) ?? h.urls[0];
        if (u) { const f = await fetchImage(u.url); img = f.buf; via = f.how; }
        else throw new Stop('shape', `${tImage.name} returned neither image data nor a URL`, { shape: shape(h.texts.map(t => { try { return JSON.parse(t); } catch { return t; } })) });
      }
    }
    if (!img) {
      const u = imageUrl(g.urls);
      if (u) { const f = await fetchImage(u.url); img = f.buf; via = f.how; }
    }
    if (!img) throw new Stop('shape', `no image for v${v.n}: ${tGen.name} gave ${screen ? `screen ${screen}` : 'no screen id'} and no image`, { shape: shape(g.texts.map(t => { try { return JSON.parse(t); } catch { return t; } })) });
    const kind = sniff(img);
    console.log(`v${v.n} image: via ${via}, ${img.length} bytes, ${kind || 'unrecognised'}, first 8 ${hex8(img)}`);
    if (!kind) throw new Stop('shape', `v${v.n}: ${via} gave ${img.length} bytes that are not PNG/JPEG/WebP/GIF (first 8 bytes ${hex8(img)})`, { via, bytes: img.length, head: hex8(img) });

    // Always JPG, quality stepped down until <= 400 KB; no PNG is kept.
    const base = sharp(img).flatten({ background: '#ffffff' });
    const meta = await sharp(img).metadata();
    let jpg;
    for (const q of QUALITIES) {
      jpg = await base.clone().jpeg({ quality: q, mozjpeg: true }).toBuffer();
      if (jpg.length <= MAX_BYTES) break;
    }
    writeFileSync(join(dir, v.file), jpg);
    results.push({ ...v, screen, via, px: `${meta.width}x${meta.height}`, kb: Math.round(jpg.length / 1024) });
    console.log(`v${v.n} ${v.key}: screen ${screen || '?'} via ${via}, ${meta.width}x${meta.height}, ${Math.round(jpg.length / 1024)} KB`);
  }

  const now = new Date().toISOString().replace(/\.\d+Z$/, 'Z');
  const index = [
    `# UI survey \`design-${cmd.tag}\``,
    '',
    `Four Google Stitch mockups of \`${cmd.screen}\` for ${repo}${cmd.app ? ` (${cmd.app})` : ''}, generated ${now} by design-options, Stitch project \`${project}\`, device ${device}. Mockups, not captures: nothing here ran the app. Prompt = brief + design language (\`${b.source}\`) + "${uiKind}" + the variant's hint; the hash is sha256 of the whole prompt (first 12 hex).`,
    '',
    'Brief:',
    '',
    ...cmd.brief.split('\n').map(l => `> ${l}`),
    '',
    '| File | Variant | Hint | Stitch screen | Pixels, size | Prompt |',
    '|---|---|---|---|---|---|',
    ...results.map(r => `| \`${r.file}\` | v${r.n} | ${r.key}: ${r.hint} | \`${r.screen || '?'}\` | ${r.px}, ${r.kb} KB | \`${r.hash}\` |`),
    '',
  ].join('\n');
  writeFileSync(join(dir, 'index.md'), index);

  await client.close().catch(() => {});
  return {
    status: 'ok', ...cmd, device, project, projectVar, created, brand: b.source, tools: names, dir: rel,
    files: results.map(({ prompt, ...r }) => r),
  };
}

try {
  finish(await main());
} catch (err) {
  const status = err instanceof Stop ? err.status : 'error';
  const message = String(err?.message ?? err).slice(0, 500);
  console.error(`design-options ${status}: ${message}`);
  if (err instanceof Stop && Object.keys(err.extra).length) console.error(JSON.stringify(err.extra).slice(0, 2000));
  finish({ status, message, ...(err instanceof Stop ? err.extra : {}), ...cmd });
}
