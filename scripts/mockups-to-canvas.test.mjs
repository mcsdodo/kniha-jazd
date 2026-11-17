// Unit tests for the mockup -> Design canvas generator (ADR-065).
// Run: node --test scripts/mockups-to-canvas.test.mjs (also part of npm run test:unit).
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { extractPage, checkUnique, renderBoard, buildCanvas, parseRegistry } from './mockups-to-canvas.mjs';

const page = (frames, extra = '') => `<!DOCTYPE html>
<html lang="en"><head><meta charset="utf-8"><title>T</title>
<style>
.frame { display: flex; }
</style>
</head>
<body>
<header class="page">
  <h1>Title</h1>
  <p>Intro text.</p>
  <div class="pagenav" id="pagenav">
    <a href="index.html">All mockups</a><a href="mapa.html">Mapa</a>
    <button class="themebtn" type="button" id="themebtn">Dark / light</button>
  </div>
</header>
<div class="frames">
${frames}
${extra}
</div>
<script>
(function () { document.getElementById("themebtn"); })();
</script>
</body></html>`;

const frame = (name, title, body = '<div class="window">x<br>y</div>') =>
  `  <section class="frame"${name ? ` data-board="${name}"` : ''}>
    <h2>${title}</h2>
    ${body}
    <ul class="ann"><li>Note.</li></ul>
  </section>`;

test('extractPage: intro, each frame in source order, open points', () => {
  const html = page(frame('p-one', '1. One') + frame('p-two', '2. Two'),
    '<section class="open-points"><h2>Open points</h2><ul class="ann"><li>Gap.</li></ul></section>');
  const p = extractPage(html, 'p');
  assert.deepEqual(p.boards.map((b) => b.name), ['p-intro', 'p-one', 'p-two', 'p-open-points']);
  assert.match(p.css, /\.frame \{ display: flex; \}/);
  assert.equal(p.boards[1].title, '1. One');
  assert.equal(p.boards[1].parentClass, 'frames');
  assert.match(p.boards[1].html, /^<section class="frame" data-board="p-one">/);
});

test('extractPage: the intro has no page nav, and no board has a script', () => {
  const p = extractPage(page(frame('p-one', '1. One')), 'p');
  const intro = p.boards[0].html;
  assert.match(intro, /Intro text\./);
  assert.doesNotMatch(intro, /index\.html|themebtn|pagenav/);
  for (const b of p.boards) assert.doesNotMatch(b.html, /<script/);
});

test('extractPage + renderBoard: a frame that uses a sprite icon carries the sprite', () => {
  const sprite = '<svg width="0" height="0" style="position:absolute" aria-hidden="true">\n  <symbol id="i-a" viewBox="0 0 24 24"><path d="M1 1"/></symbol>\n</svg>\n';
  const html = page(frame('p-icon', '1. Icon', '<svg class="ic"><use href="#i-a"/></svg>') + frame('p-plain', '2. Plain'))
    .replace('<div class="frames">', sprite + '<div class="frames">');
  const p = extractPage(html, 'p');
  const [icon, plain] = p.boards.slice(1);
  assert.match(renderBoard(p, icon, { w: 10, h: 10, innerW: 2 }), /<symbol id="i-a"/);
  assert.doesNotMatch(renderBoard(p, plain, { w: 10, h: 10, innerW: 2 }), /<symbol/);
});

test('extractPage: a frame without data-board fails', () => {
  assert.throws(() => extractPage(page(frame('', '3. Nameless')), 'p'), /data-board.*3\. Nameless/);
});

test('extractPage: an unclosed element in a frame fails', () => {
  const bad = frame('p-bad', '1. Bad', '<div class="window"><span>open</div>');
  assert.throws(() => extractPage(page(bad), 'p'), /p-bad.*span/);
});

test('checkUnique: a board name in two pages fails', () => {
  const a = extractPage(page(frame('x-same', '1. A')), 'a');
  const b = extractPage(page(frame('x-same', '1. B')), 'b');
  assert.throws(() => checkUnique([a, b]), /x-same/);
  assert.doesNotThrow(() => checkUnique([a]));
});

test('renderBoard: a valid Design Component page with the measured size', () => {
  const p = extractPage(page(frame('p-one', '1. One')), 'p');
  const out = renderBoard(p, p.boards[1], { w: 900, h: 700, innerW: 852 });
  assert.match(out, /<script src="\.\/support\.js"><\/script>/);
  assert.match(out, /<x-dc>\s*<helmet>\s*<style>[\s\S]*body\{margin:0\}[\s\S]*\.frame \{ display: flex; \}/);
  assert.match(out, /<div class="frames" style="width: 900px; height: 700px;/);
  assert.match(out, /data-board="p-one" style="width: 852px"/);
  assert.match(out, /data-props='\{"\$preview":\{"width":900,"height":700\}\}'/);
  assert.match(out, /class Component extends DCLogic/);
  assert.match(out, /<title>1\. One<\/title>/);
});

const sizes = { 'p-intro': { w: 500, h: 200 }, 'p-one': { w: 900, h: 700 }, 'p-two': { w: 900, h: 400 } };
const pages = () => [{ id: 'p', name: 'Page P', ...extractPage(page(frame('p-one', '1. One') + frame('p-two', '2. Two')), 'p') }];

test('buildCanvas: one canvas page per file, a board entry and an order slot per board', () => {
  const { canvas, removed } = buildCanvas(pages(), sizes, { title: 'T', now: '2026-11-16T00:00:00Z' });
  assert.equal(canvas.v, 3);
  assert.deepEqual(canvas.createdOnFiles, { v: 1, at: '2026-11-16T00:00:00Z' });
  assert.deepEqual(canvas.pages, [{ id: 'p', name: 'Page P' }]);
  assert.deepEqual(canvas.order, ['p-intro.dc.html', 'p-one.dc.html', 'p-two.dc.html']);
  const one = canvas.boards['p-one.dc.html'];
  assert.equal(one.page, 'p');
  assert.equal(one.w, 900);
  assert.equal(one.title, '1. One');
  assert.equal(canvas.notes['gen-p'].kind, 'title1');
  assert.ok(canvas.notes['gen-p'].y <= canvas.boards['p-intro.dc.html'].y - 223);
  assert.deepEqual(removed, []);
});

test('buildCanvas: a merge keeps user notes and editor keys, and reports removed boards', () => {
  const live = {
    v: 3, title: 'Kept title', createdOnFiles: { v: 1, at: '2026-01-01T00:00:00Z' }, extraKey: 1,
    boards: {
      'p-one.dc.html': { x: 1, y: 2, w: 3, h: 4, frameless: true },
      'p-gone.dc.html': { x: 0, y: 0, w: 10, h: 10 },
    },
    order: ['p-gone.dc.html', 'p-one.dc.html'],
    notes: { mine: { x: 5, y: 5, text: 'User sticky' }, 'gen-old': { x: 0, y: 0, text: 'Old' } },
  };
  const { canvas, removed } = buildCanvas(pages(), sizes, { title: 'T', now: 'N', live });
  assert.equal(canvas.title, 'Kept title');
  assert.equal(canvas.extraKey, 1);
  assert.deepEqual(canvas.createdOnFiles, live.createdOnFiles);
  assert.equal(canvas.boards['p-one.dc.html'].frameless, true);
  assert.equal(canvas.boards['p-one.dc.html'].w, 900);
  assert.equal(canvas.boards['p-gone.dc.html'], undefined);
  assert.deepEqual(removed, ['p-gone.dc.html']);
  assert.equal(canvas.notes.mine.text, 'User sticky');
  assert.equal(canvas.notes['gen-old'], undefined);
});

test('parseRegistry: page names and files between the markers', () => {
  const md = `x\n<!-- registry:start -->\n| Page | File | Source paths |\n|---|---|---|\n| Mobil (\`/m\`) | [mobil.html](mobil.html) | \`src/routes/m\` |\n| Jazdy (\`/\`, header) | [jazdy.html](jazdy.html) | \`a\` |\n<!-- registry:end -->\n`;
  assert.deepEqual(parseRegistry(md), [
    { id: 'mobil', name: 'Mobil (/m)', file: 'mobil.html' },
    { id: 'jazdy', name: 'Jazdy (/, header)', file: 'jazdy.html' },
  ]);
});
