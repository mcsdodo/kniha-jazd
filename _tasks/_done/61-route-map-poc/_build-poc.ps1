$ErrorActionPreference = "Stop"
$here = $PSScriptRoot
$villages = Get-Content "$here\villages.json" -Raw -Encoding utf8
$matrix = Get-Content "$here\matrix.json" -Raw -Encoding utf8

$template = @'
<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8">
<title>Route Map POC — kniha jázd</title>
<link rel="stylesheet" href="https://unpkg.com/leaflet@1.9.4/dist/leaflet.css">
<style>
  html, body { margin: 0; padding: 0; height: 100%; font-family: system-ui, -apple-system, sans-serif; }
  #app { display: flex; flex-direction: column; height: 100vh; }
  #controls {
    padding: 10px 14px; background: #f4f4f4; border-bottom: 1px solid #ccc;
    display: flex; gap: 14px; align-items: center; flex-wrap: wrap; font-size: 14px;
  }
  #controls label { display: inline-flex; align-items: center; gap: 6px; }
  #controls input, #controls select { padding: 4px 8px; font-size: 14px; border: 1px solid #bbb; border-radius: 3px; }
  #controls input[type=number] { width: 80px; }
  #controls button {
    padding: 5px 14px; font-size: 14px; background: #0066cc; color: white;
    border: 0; border-radius: 3px; cursor: pointer;
  }
  #controls button:hover { background: #0052a3; }
  #controls button:disabled { background: #999; cursor: wait; }
  #status { font-size: 13px; }
  .err { color: #c00; }
  .warn { color: #b87000; }
  .ok { color: #060; }
  #info {
    padding: 8px 14px; background: white; border-bottom: 1px solid #ddd;
    font-size: 13px; min-height: 18px; line-height: 1.5;
  }
  #info code { background: #eef; padding: 1px 4px; border-radius: 2px; }
  #map { flex: 1; }
  #sessions { display: flex; gap: 6px; flex-wrap: wrap; }
  #sessions button {
    padding: 3px 10px; font-size: 12px; background: #ddd; color: #333; border: 1px solid #bbb; border-radius: 3px;
  }
  #sessions button.active { background: #0066cc; color: white; border-color: #0066cc; }
</style>
</head>
<body>
<div id="app">
  <div id="controls">
    <label>Target km: <input type="number" id="target" value="100" min="5" max="2000" step="1"></label>
    <label>Max stops: <input type="number" id="maxStops" value="5" min="1" max="10" step="1"></label>
    <button id="generate">Generate</button>
    <span id="status"></span>
    <span id="sessions"></span>
  </div>
  <div id="info">Pick a target km and an algorithm, then click <strong>Generate</strong>.</div>
  <div id="map"></div>
</div>
<script src="https://unpkg.com/leaflet@1.9.4/dist/leaflet.js"></script>
<script>
const VILLAGES = __VILLAGES__;
const MATRIX = __MATRIX__;

const HOME = 0;
const N = VILLAGES.nodes.length;
const TOLERANCE = 0.05;
const SINGLE_SESSION_MAX_KM = 400;

let MAX_STOPS = 5;

function totalKm(seq) {
  let t = 0;
  for (let i = 0; i < seq.length - 1; i++) t += MATRIX.distances[seq[i]][seq[i+1]];
  return t;
}

// --- Genetic algorithm ---
function generateGA(targetKm) {
  const POP = 50, GENS = 100, MUT = 0.25, ELITE = 2, TOUR = 3;
  const fitness = c => 1 / (1 + Math.abs(totalKm(c) - targetKm));
  const randVillage = () => 1 + Math.floor(Math.random() * (N - 1));
  const randInt = n => Math.floor(Math.random() * n);

  function makeCh() {
    const k = 1 + randInt(MAX_STOPS);
    const v = [];
    while (v.length < k) {
      const x = randVillage();
      if (!v.includes(x)) v.push(x);
    }
    return [HOME, ...v, HOME];
  }
  function tournament(pop, fits) {
    let best = -1, bf = -1;
    for (let i = 0; i < TOUR; i++) {
      const idx = randInt(pop.length);
      if (fits[idx] > bf) { bf = fits[idx]; best = idx; }
    }
    return pop[best];
  }
  function crossover(p1, p2) {
    const m1 = p1.slice(1, -1), m2 = p2.slice(1, -1);
    if (m1.length === 0) return [...p2];
    if (m2.length === 0) return [...p1];
    const start = randInt(m1.length);
    const end = start + randInt(m1.length - start);
    const slice = m1.slice(start, end + 1);
    const child = [...slice];
    for (const v of m2) {
      if (!child.includes(v) && child.length < MAX_STOPS) child.push(v);
    }
    return [HOME, ...child, HOME];
  }
  function mutate(c) {
    if (Math.random() > MUT) return c;
    const m = c.slice(1, -1);
    const op = Math.random();
    if (op < 0.34 && m.length < MAX_STOPS) {
      let v;
      do { v = randVillage(); } while (m.includes(v));
      m.splice(randInt(m.length + 1), 0, v);
    } else if (op < 0.67 && m.length > 1) {
      m.splice(randInt(m.length), 1);
    } else if (m.length >= 2) {
      const i = randInt(m.length), j = randInt(m.length);
      [m[i], m[j]] = [m[j], m[i]];
    }
    return [HOME, ...m, HOME];
  }

  let pop = Array.from({length: POP}, makeCh);
  for (let g = 0; g < GENS; g++) {
    const fits = pop.map(fitness);
    const sorted = pop.map((c, i) => ({c, f: fits[i]})).sort((a, b) => b.f - a.f);
    const next = sorted.slice(0, ELITE).map(s => s.c);
    while (next.length < POP) {
      const a = tournament(pop, fits);
      const b = tournament(pop, fits);
      next.push(mutate(crossover(a, b)));
    }
    pop = next;
  }
  const fits = pop.map(fitness);
  let best = 0;
  for (let i = 1; i < pop.length; i++) if (fits[i] > fits[best]) best = i;
  return { sequence: pop[best], totalKm: totalKm(pop[best]) };
}

// --- Multi-session split ---
function planSessions(target) {
  if (target <= SINGLE_SESSION_MAX_KM) return [target];
  const n = Math.ceil(target / SINGLE_SESSION_MAX_KM);
  return Array(n).fill(target / n);
}

// --- OSRM polyline fetch ---
async function fetchPolyline(seq) {
  const coords = seq.map(i => {
    const n = VILLAGES.nodes[i];
    return n.lon + ',' + n.lat;
  }).join(';');
  const url = 'https://router.project-osrm.org/route/v1/driving/' + coords +
              '?geometries=geojson&overview=full&steps=false';
  const r = await fetch(url);
  if (!r.ok) throw new Error('OSRM HTTP ' + r.status);
  const d = await r.json();
  if (d.code !== 'Ok') throw new Error('OSRM: ' + d.code);
  return { geometry: d.routes[0].geometry, actualKm: d.routes[0].distance / 1000 };
}

// --- UI / Render ---
const map = L.map('map', { zoomControl: true })
  .setView([VILLAGES.homeCoords.lat, VILLAGES.homeCoords.lon], 10);
L.tileLayer('https://tile.openstreetmap.org/{z}/{x}/{y}.png', {
  maxZoom: 19,
  attribution: '&copy; <a href="https://www.openstreetmap.org/copyright">OpenStreetMap</a> contributors'
}).addTo(map);

let routeLayer = null;
let cachedSessions = [];

async function renderSession(idx) {
  const s = cachedSessions[idx];
  if (!s) return;
  const info = document.getElementById('info');
  const status = document.getElementById('status');

  if (routeLayer) { map.removeLayer(routeLayer); routeLayer = null; }

  status.textContent = 'Fetching polyline…'; status.className = '';
  try {
    const poly = await fetchPolyline(s.result.sequence);
    routeLayer = L.geoJSON(poly.geometry, {
      style: { color: '#0066cc', weight: 5, opacity: 0.85 }
    }).addTo(map);
    map.fitBounds(routeLayer.getBounds(), { padding: [30, 30] });

    const matrixKm = s.result.totalKm;
    const targetKm = s.target;
    const errPct = ((matrixKm - targetKm) / targetKm) * 100;
    const stopNames = s.result.sequence.slice(1, -1).map(i => VILLAGES.nodes[i].name);

    info.innerHTML =
      'Time: ' + s.elapsedMs + 'ms · ' +
      'Matrix: ' + matrixKm.toFixed(1) + ' km · ' +
      'Target: ' + targetKm.toFixed(1) + ' km · ' +
      'Error: <span class="' + (Math.abs(errPct) < 5 ? 'ok' : 'warn') + '">' +
      (errPct >= 0 ? '+' : '') + errPct.toFixed(1) + '%</span>' +
      '<br>Stops (' + stopNames.length + '): ' + (stopNames.length ? stopNames.join(' → ') : '(direct)') +
      '<br><span class="ok">OSRM polyline: ' + poly.actualKm.toFixed(1) + ' km road distance</span>';
    status.textContent = '';
  } catch (e) {
    status.textContent = 'Error: ' + e.message;
    status.className = 'err';
  }
}

function renderSessionTabs(active) {
  const el = document.getElementById('sessions');
  el.innerHTML = '';
  if (cachedSessions.length <= 1) return;
  cachedSessions.forEach((s, i) => {
    const b = document.createElement('button');
    b.textContent = 'Session ' + (i + 1) + '/' + cachedSessions.length + ' (' + s.target.toFixed(0) + ' km)';
    if (i === active) b.classList.add('active');
    b.addEventListener('click', () => { renderSessionTabs(i); renderSession(i); });
    el.appendChild(b);
  });
}

async function generate() {
  const target = parseFloat(document.getElementById('target').value);
  MAX_STOPS = Math.max(1, Math.min(10, parseInt(document.getElementById('maxStops').value, 10) || 5));
  const status = document.getElementById('status');
  const btn = document.getElementById('generate');

  if (isNaN(target) || target < 5) {
    status.textContent = 'Target too short (min 5 km)';
    status.className = 'err';
    return;
  }

  btn.disabled = true;
  status.textContent = 'Running GA…'; status.className = '';

  const sessions = planSessions(target);
  cachedSessions = [];
  for (const t of sessions) {
    const t0 = performance.now();
    const result = generateGA(t);
    const elapsedMs = Math.round(performance.now() - t0);
    cachedSessions.push({ target: t, result, elapsedMs });
  }

  renderSessionTabs(0);
  await renderSession(0);
  btn.disabled = false;
}

document.getElementById('generate').addEventListener('click', generate);
document.getElementById('target').addEventListener('keypress', e => { if (e.key === 'Enter') generate(); });
</script>
</body>
</html>
'@

$output = $template.Replace('__VILLAGES__', $villages.Trim()).Replace('__MATRIX__', $matrix.Trim())
$out = "$here\poc.html"
Set-Content -Path $out -Value $output -Encoding utf8 -NoNewline
$bytes = (Get-Item $out).Length
Write-Host "Wrote $out ($([Math]::Round($bytes/1024, 1)) KB)"
