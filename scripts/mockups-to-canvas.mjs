#!/usr/bin/env node
// Generate the claude.ai Design canvas from the mockup files in docs/mockups/ (ADR-065).
//
// The HTML files are the source. The canvas is a build output: one artboard
// (project/<board>.dc.html) per frame, plus project/canvas.json. Never edit the canvas by hand.
//
//   node scripts/mockups-to-canvas.mjs --out <dir> [--pages mobil,jazdy] [--live <canvas.json>]
//
// --live merges with the canvas.json read from the published canvas: user notes and editor
// keys stay. The script prints the files to publish and the boards to remove (send as null).
// No dependencies, so that `npm run test:unit` runs without `npm ci`. Only the CLI part
// needs webdriverio and Chrome, to measure each frame in the source file.
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const VOID = new Set('area base br col embed hr img input link meta source track wbr'.split(' '));
const PAD = 24; // px around a frame inside its artboard
const GAP_X = 80;
const GAP_Y = 120;
const ROW_MAX = 4200; // a row of boards wraps after this width
const TITLE_ABOVE = 300; // the page title note sits this far above the first row

// Parse the HTML into element nodes with their source ranges. Comments, <script> and <style>
// bodies are skipped. Throws on a close tag that does not match (an unclosed element).
function parseTree(src, label) {
  const root = { tag: '#root', attrs: '', start: 0, end: src.length, children: [] };
  const stack = [root];
  const re = /<!--[\s\S]*?-->|<\/?([a-zA-Z][a-zA-Z0-9-]*)\b([^>]*)>/g;
  let m;
  while ((m = re.exec(src))) {
    if (m[0].startsWith('<!--')) continue;
    const tag = m[1].toLowerCase();
    if (m[0][1] === '/') {
      const top = stack[stack.length - 1];
      if (top.tag !== tag) {
        const board = [...stack].reverse().map((n) => n.attrs && n.attrs.match(/data-board="([^"]*)"/)).find(Boolean);
        const where = board ? ` in board ${board[1]}` : '';
        throw new Error(`${label}${where}: <${top.tag}> is not closed before </${tag}> (offset ${m.index})`);
      }
      top.end = m.index + m[0].length;
      stack.pop();
      continue;
    }
    const node = { tag, attrs: m[2], start: m.index, openEnd: m.index + m[0].length, children: [] };
    stack[stack.length - 1].children.push(node);
    if (VOID.has(tag) || m[2].trim().endsWith('/')) {
      node.end = node.openEnd;
    } else if (tag === 'script' || tag === 'style') {
      const close = src.indexOf(`</${tag}>`, node.openEnd);
      node.end = close + tag.length + 3;
      re.lastIndex = node.end;
    } else {
      stack.push(node);
    }
  }
  if (stack.length > 1) throw new Error(`${label}: <${stack[stack.length - 1].tag}> is not closed`);
  return root;
}

const attr = (node, name) => {
  const m = node.attrs.match(new RegExp(`(?:^|\\s)${name}="([^"]*)"`));
  return m ? m[1] : null;
};
const hasClass = (node, cls) => (attr(node, 'class') || '').split(/\s+/).includes(cls);

function* walk(node) {
  for (const c of node.children) {
    yield c;
    yield* walk(c);
  }
}

// Source text of a node, with the ranges of some descendants cut out.
function sliceWithout(src, node, cut) {
  let out = '';
  let pos = node.start;
  for (const c of [...cut].sort((a, b) => a.start - b.start)) {
    out += src.slice(pos, c.start);
    pos = c.end;
  }
  return out + src.slice(pos, node.end);
}

const text = (src, node) => src.slice(node.openEnd, node.end).replace(/<[^>]+>/g, '').replace(/\s+/g, ' ').trim();

function checkBoard(html, name) {
  parseTree(html, name);
  const unquoted = html.match(/<[a-zA-Z][^>]*\s[a-zA-Z-]+=[^"'\s>][^>]*>/);
  if (unquoted) throw new Error(`${name}: unquoted attribute in ${unquoted[0]}`);
  if (html.includes('{{')) throw new Error(`${name}: "{{" would be a Design Component hole`);
}

// One mockup file -> its shared CSS and its boards: the intro, each frame, the open points.
export function extractPage(src, stem) {
  const tree = parseTree(src, `${stem}.html`);
  const all = [...walk(tree)];
  const style = all.find((n) => n.tag === 'style');
  const css = style ? src.slice(style.openEnd, style.end - '</style>'.length).trim() : '';
  const boards = [];

  const header = all.find((n) => n.tag === 'header' && hasClass(n, 'page'));
  if (header) {
    // The page nav links to the other files and holds the theme button; neither works on a canvas.
    const navLink = [...walk(header)].find((n) => n.tag === 'a' && attr(n, 'href') === 'index.html');
    const nav = navLink && [...walk(header)].filter((n) => n.children.includes(navLink))[0];
    const html = sliceWithout(src, header, nav ? [nav] : []);
    const h1 = [...walk(header)].find((n) => n.tag === 'h1');
    boards.push({ name: `${stem}-intro`, title: h1 ? text(src, h1) : stem, parentClass: '', html });
  }

  for (const n of all) {
    if (n.tag !== 'section') continue;
    const h2 = n.children.find((c) => c.tag === 'h2');
    const title = h2 ? text(src, h2) : '(no h2)';
    let name;
    if (hasClass(n, 'open-points')) name = `${stem}-open-points`;
    else if (hasClass(n, 'frame') || hasClass(n, 'screen')) {
      name = attr(n, 'data-board');
      if (!name) throw new Error(`${stem}.html: no data-board attribute on the frame "${title}"`);
    } else continue;
    const parent = all.find((p) => p.children.includes(n)) || tree;
    boards.push({ name, title, parentClass: attr(parent, 'class') || '', html: src.slice(n.start, n.end) });
  }

  // An icon sprite (<svg> with <symbol>s) outside the frames: a board that uses an icon
  // (<use href="#id">) needs it, because each board renders alone.
  const sprites = all
    .filter((n) => n.tag === 'svg' && [...walk(n)].some((c) => c.tag === 'symbol'))
    .map((n) => src.slice(n.start, n.end));
  for (const b of boards) {
    checkBoard(b.html, b.name);
    if (sprites.length && /<use\s[^>]*href="#/.test(b.html)) b.defs = sprites.join('\n');
  }
  return { id: stem, css, boards };
}

export function checkUnique(pages) {
  const seen = new Map();
  for (const p of pages) {
    for (const b of p.boards) {
      if (seen.has(b.name)) throw new Error(`board name "${b.name}" is in ${seen.get(b.name)} and in ${p.id}`);
      seen.set(b.name, p.id);
    }
  }
}

const esc = (s) => s.replace(/&/g, '&amp;').replace(/</g, '&lt;');

// One board -> a Design Component page. The frame keeps its measured width, so the layout
// matches the source file; the root has the fixed board size.
export function renderBoard(page, board, size) {
  const cls = board.parentClass ? ` class="${board.parentClass}"` : '';
  const inner = board.html.replace(/^<(\w+)([^>]*)>/, (m, tag, attrs) => {
    if (/\sstyle="/.test(attrs)) return `<${tag}${attrs.replace(/\sstyle="/, ` style="width: ${size.innerW}px; `)}>`;
    return `<${tag}${attrs} style="width: ${size.innerW}px">`;
  });
  const props = JSON.stringify({ $preview: { width: size.w, height: size.h } });
  return `<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<title>${esc(board.title)}</title>
<script src="./support.js"></script>
</head>
<body>
<!-- Generated by scripts/mockups-to-canvas.mjs from docs/mockups/${page.id}.html. Do not edit. -->
<x-dc>
<helmet>
<style>
body{margin:0}
${page.css}
</style>
</helmet>
<div${cls} style="width: ${size.w}px; height: ${size.h}px; box-sizing: border-box; padding: ${PAD}px; margin: 0; max-width: none; display: block; overflow: hidden; background: var(--page-bg, var(--bg))">
${board.defs ? board.defs + '\n' : ''}${inner}
</div>
</x-dc>
<script type="text/x-dc" data-dc-script data-props='${props}'>
class Component extends DCLogic {
renderVals() {
return {};
}
}
</script>
</body>
</html>
`;
}

// The canvas index. Each mockup file is a canvas page; its boards wrap in rows under a title.
// With opts.live, keys and notes that the generator does not own stay.
export function buildCanvas(pages, sizes, { title, now, live } = {}) {
  const genPages = new Set(pages.map((p) => p.id));
  const base = live ? structuredClone(live) : { v: 3, createdOnFiles: { v: 1, at: now }, title };
  const oldBoards = base.boards || {};
  const boards = {};
  const order = [];
  const notes = {};

  // Boards of pages that this run does not generate stay as they are.
  for (const [key, b] of Object.entries(oldBoards)) {
    if (b.page && !genPages.has(b.page)) boards[key] = b;
  }
  for (const key of base.order || []) if (boards[key]) order.push(key);
  for (const [id, n] of Object.entries(base.notes || {})) {
    if (!id.startsWith('gen-') || (n.page && !genPages.has(n.page))) notes[id] = n;
  }

  for (const p of pages) {
    let x = 0;
    let y = 0;
    let rowH = 0;
    let maxW = 0;
    for (const b of p.boards) {
      const s = sizes[b.name];
      if (!s) throw new Error(`no size for board ${b.name}`);
      if (x > 0 && x + s.w > ROW_MAX) {
        x = 0;
        y += rowH + GAP_Y;
        rowH = 0;
      }
      const key = `${b.name}.dc.html`;
      boards[key] = { ...(oldBoards[key] || {}), x, y, w: s.w, h: s.h, title: b.title, page: p.id };
      order.push(key);
      maxW = Math.max(maxW, x + s.w);
      x += s.w + GAP_X;
      rowH = Math.max(rowH, s.h);
    }
    notes[`gen-${p.id}`] = { x: 0, y: -TITLE_ABOVE, text: p.name, kind: 'title1', maxW, page: p.id };
  }

  const removed = Object.keys(oldBoards).filter((k) => !boards[k]);
  const otherPages = (base.pages || []).filter((pg) => !genPages.has(pg.id));
  const canvas = {
    ...base,
    pages: [...pages.map((p) => ({ id: p.id, name: p.name })), ...otherPages],
    boards,
    order,
    notes,
  };
  if (!canvas.launch) canvas.launch = { view: 'canvas', page: pages[0].id };
  if (!canvas.designSystems) canvas.designSystems = [];
  return { canvas, removed };
}

// The registry table in docs/mockups/README.md: page name and file per row.
export function parseRegistry(md) {
  const body = md.split('<!-- registry:start -->')[1].split('<!-- registry:end -->')[0];
  const rows = [];
  for (const line of body.split('\n')) {
    const m = line.match(/^\|\s*([^|]+?)\s*\|\s*\[([^\]]+)\]\([^)]+\)\s*\|/);
    if (!m) continue;
    rows.push({ id: m[2].replace(/\.html$/, ''), name: m[1].replace(/`/g, ''), file: m[2] });
  }
  return rows;
}

// Measure each board in the source file in headless Chrome, through webdriverio (a
// devDependency that the integration tests use too).
async function measure(files) {
  const { remote } = await import('webdriverio');
  const browser = await remote({
    logLevel: 'error',
    capabilities: {
      browserName: 'chrome',
      'goog:chromeOptions': { args: ['--headless=new', '--no-sandbox', '--window-size=1440,900', '--force-prefers-color-scheme=light'] },
    },
  });
  const sizes = {};
  try {
    for (const { file, page } of files) {
      await browser.url(`file://${file}`);
      const rects = await browser.execute((stem) => {
        const out = {};
        // A flex row stretches its frames to the tallest one; measure each at its own height.
        for (const el of document.querySelectorAll('[data-board], section.open-points')) {
          const ps = getComputedStyle(el.parentElement);
          if (ps.display.includes('flex') && ps.flexDirection.startsWith('row')) el.style.alignSelf = 'flex-start';
        }
        const put = (name, el) => {
          if (!el) return;
          const r = el.getBoundingClientRect();
          out[name] = { width: r.width, height: r.height };
        };
        put(`${stem}-intro`, document.querySelector('header.page'));
        put(`${stem}-open-points`, document.querySelector('section.open-points'));
        for (const el of document.querySelectorAll('[data-board]')) put(el.getAttribute('data-board'), el);
        return out;
      }, page.id);
      for (const [name, r] of Object.entries(rects)) {
        const innerW = Math.ceil(r.width);
        sizes[name] = { innerW, w: innerW + 2 * PAD, h: Math.ceil(r.height) + 2 * PAD + 8 };
      }
    }
  } finally {
    await browser.deleteSession();
  }
  return sizes;
}

async function main(argv) {
  const arg = (k) => {
    const i = argv.indexOf(k);
    return i >= 0 ? argv[i + 1] : undefined;
  };
  const out = arg('--out');
  if (!out) throw new Error('usage: mockups-to-canvas.mjs --out <dir> [--pages a,b] [--live <canvas.json>]');
  const repo = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
  const dir = path.join(repo, 'docs/mockups');
  const registry = parseRegistry(fs.readFileSync(path.join(dir, 'README.md'), 'utf8'));
  const only = arg('--pages')?.split(',');
  const rows = only ? registry.filter((r) => only.includes(r.id)) : registry;
  if (only && rows.length !== only.length) throw new Error(`unknown page in --pages ${only}`);

  // Names must be unique on the whole canvas, so check all pages, not only the selected ones.
  const allPages = registry.map((r) => ({
    ...extractPage(fs.readFileSync(path.join(dir, r.file), 'utf8'), r.id),
    name: r.name,
    file: path.join(dir, r.file),
  }));
  checkUnique(allPages);
  const pages = allPages.filter((p) => rows.some((r) => r.id === p.id));

  const sizes = await measure(pages.map((p) => ({ file: p.file, page: p })));
  const live = arg('--live') ? JSON.parse(fs.readFileSync(arg('--live'), 'utf8')) : undefined;
  const { canvas, removed } = buildCanvas(pages, sizes, {
    title: 'Kniha jázd: UI mockups',
    now: new Date().toISOString().replace(/\.\d+Z$/, 'Z'),
    live,
  });

  const proj = path.join(out, 'project');
  fs.mkdirSync(proj, { recursive: true });
  const files = [];
  for (const p of pages) {
    for (const b of p.boards) {
      fs.writeFileSync(path.join(proj, `${b.name}.dc.html`), renderBoard(p, b, sizes[b.name]));
      files.push(`project/${b.name}.dc.html`);
    }
  }
  fs.writeFileSync(path.join(proj, 'canvas.json'), JSON.stringify(canvas, null, 2) + '\n');
  console.log(JSON.stringify({ root: path.resolve(out), files, removed: removed.map((k) => `project/${k}`) }, null, 2));
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  main(process.argv.slice(2)).catch((e) => {
    console.error(e.message);
    process.exit(1);
  });
}
