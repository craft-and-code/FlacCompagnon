/* FlacCompagnon landing — vanilla i18n, interactive tooltips and canvas
   animations. Ported from the design handoff (no framework). */

import { DICT } from "./copy.js";

let lang = "fr";
const $ = (s) => document.querySelector(s);
const specState = { key: "" };
const docsRoot = document.documentElement.dataset.docsRoot || "docs";

function applyLang(l) {
  lang = DICT[l] ? l : "fr";
  const t = DICT[lang];
  document.querySelectorAll("[data-i18n]").forEach((el) => {
    const v = t[el.getAttribute("data-i18n")];
    if (v != null) el.textContent = v;
  });
  // Keys that contain markup (e.g. <code>) use innerHTML — trusted, own content.
  document.querySelectorAll("[data-i18n-html]").forEach((el) => {
    const v = t[el.getAttribute("data-i18n-html")];
    if (v != null) el.innerHTML = v;
  });
  document.querySelectorAll("[data-fr][data-en]").forEach((el) => {
    el.textContent = el.dataset[lang];
  });
  document.querySelectorAll("[data-doc]").forEach((el) => {
    el.href = `${docsRoot}/${lang}/${el.dataset.doc}.html`;
  });
  document.documentElement.lang = lang;
  $("#lang-fr").classList.toggle("on", lang === "fr");
  $("#lang-en").classList.toggle("on", lang === "en");
  $("#lang-fr").setAttribute("aria-pressed", String(lang === "fr"));
  $("#lang-en").setAttribute("aria-pressed", String(lang === "en"));
  specState.key = ""; // force spectrogram redraw (caption localization)
  try { localStorage.setItem("fc-lang", lang); } catch {}
}

// --- language toggle ---
$("#lang-fr").addEventListener("click", () => applyLang("fr"));
$("#lang-en").addEventListener("click", () => applyLang("en"));
// The English landing page selects its language from its stable URL. The root
// page otherwise follows the browser language, and a previous manual choice
// still wins there.
const pageLanguage = document.documentElement.dataset.defaultLanguage;
let initial = pageLanguage || ((navigator.language || "").toLowerCase().startsWith("fr") ? "fr" : "en");
if (!pageLanguage) {
  try {
    const saved = localStorage.getItem("fc-lang");
    if (saved === "fr" || saved === "en") initial = saved;
  } catch {}
}
applyLang(initial);

// --- floating tooltip for the app-preview table tags ---
const ftip = document.createElement("div");
ftip.id = "floattip";
document.body.appendChild(ftip);
document.querySelectorAll("[data-tip],[data-path]").forEach((el) => {
  el.addEventListener("mouseenter", () => {
    const key = el.getAttribute("data-tip");
    ftip.textContent = key ? (DICT[lang][key] || "") : (el.getAttribute("data-path") || "");
    ftip.style.opacity = "1";
  });
  el.addEventListener("mousemove", (e) => {
    const pad = 14;
    let x = e.clientX + pad, y = e.clientY + pad;
    const r = ftip.getBoundingClientRect();
    if (x + r.width > window.innerWidth - 8) x = e.clientX - r.width - pad;
    if (y + r.height > window.innerHeight - 8) y = e.clientY - r.height - pad;
    ftip.style.left = x + "px";
    ftip.style.top = y + "px";
  });
  el.addEventListener("mouseleave", () => { ftip.style.opacity = "0"; });
});

/* =========================== Canvas animations ============================= */
const INTENSITY = 6;
const BG_STYLE = "bars";
const T = () => DICT[lang];

function rng(seed) {
  let s = seed >>> 0;
  return () => { s = (s * 1664525 + 1013904223) >>> 0; return s / 4294967296; };
}
function sizeCanvas(c, dpr) {
  const w = Math.round(c.clientWidth * dpr), h = Math.round(c.clientHeight * dpr);
  if (c.width !== w || c.height !== h) { c.width = w; c.height = h; return true; }
  return false;
}
// Scroll window over which a diagram animates, as fractions of the viewport
// height measured on the *canvas itself* — not on its card. The canvas is
// pinned to the bottom of the card, so a card-based measure would run most of
// the animation while the drawing is still off-screen. Starting at 0.95 means
// it begins as the canvas first peeks in from the bottom; ending at 0.52 means
// it is fully played once the canvas sits mid-screen, which also puts the
// card's badge roughly 100 px below the sticky nav.
const ANIM_START_VH = 0.95;
const ANIM_END_VH = 0.52;

// The conversion diagram gets a longer window than the detection ones. Those
// draw a single continuous picture, so 0.43 vh of scrolling is enough to read
// them; this one has six tree rows arriving in sequence, and over the same
// distance the whole sequence was over before the reader had settled on the
// section. Starting at 1.0 (the canvas has not quite entered yet) and ending
// at 0.18 (it is near the top of the viewport, still fully visible) roughly
// doubles the travel, so each row gets its own moment.
const CONV_START_VH = 1.0;
const CONV_END_VH = 0.18;

function progress(c, startVh, endVh) {
  const r = c.getBoundingClientRect(), vh = window.innerHeight;
  const start = vh * (startVh === undefined ? ANIM_START_VH : startVh);
  const end = vh * (endVh === undefined ? ANIM_END_VH : endVh);
  return Math.max(0, Math.min(1, (start - r.top) / (start - end)));
}
function visible(c) {
  const r = c.getBoundingClientRect();
  return r.bottom > -60 && r.top < window.innerHeight + 60;
}

let hm = null;
function drawBg(c, ts, k) {
  const ctx = c.getContext("2d"), w = c.width, h = c.height;
  const sc = (window.scrollY || 0) * 0.0035;
  ctx.clearRect(0, 0, w, h);
  if (BG_STYLE === "bars") {
    const n = 72, bw = w / n;
    const grad = ctx.createLinearGradient(0, h, 0, h * 0.4);
    grad.addColorStop(0, "rgba(75,130,240,0.26)");
    grad.addColorStop(1, "rgba(123,79,240,0.06)");
    ctx.fillStyle = grad;
    for (let i = 0; i < n; i++) {
      const env = Math.exp((-i / n) * 2.0) * 0.7 + 0.3;
      const v = (Math.sin(i * 0.55 + ts * 0.0009 + sc * 2) + Math.sin(i * 0.23 - ts * 0.00045 + sc)) * 0.25 + 0.5;
      const bh = h * 0.44 * env * Math.max(0.06, v) * k;
      ctx.fillRect(i * bw + bw * 0.22, h - bh, bw * 0.56, bh);
    }
  }
}

function drawUp(c, p, t) {
  const ctx = c.getContext("2d"), w = c.width, h = c.height;
  ctx.clearRect(0, 0, w, h);
  const lanes = 24, lh = h / lanes;
  const cols = Math.floor(w / (10 * (w / c.clientWidth)));
  const cw = w / cols, rnd = rng(42);
  for (let lane = 0; lane < lanes; lane++) {
    const active = lane < 16;
    const reveal = Math.max(0, Math.min(1, p * 30 - lane));
    for (let col = 0; col < cols; col++) {
      const r = rnd();
      const y = lane * lh + lh * 0.18, bh = lh * 0.64;
      const x = col * cw + cw * 0.12, bwd = cw * 0.76;
      if (active && r < 0.52) {
        const mix = lane / 16;
        const cr = Math.round(90 + 33 * mix), cg = Math.round(130 - 51 * mix);
        ctx.fillStyle = "rgba(" + cr + "," + cg + ",240," + (0.25 + r * 0.55) * reveal + ")";
        ctx.fillRect(x, y, bwd, bh);
      } else {
        ctx.fillStyle = active ? "rgba(139,147,167,0.06)" : "rgba(139,147,167,0.05)";
        ctx.fillRect(x, y, bwd, bh);
      }
    }
  }
  const by = (16 / 24) * h, s = w / c.clientWidth;
  ctx.strokeStyle = "rgba(239,91,106,0.85)";
  ctx.lineWidth = Math.max(1, s);
  ctx.setLineDash([6, 5]);
  ctx.beginPath(); ctx.moveTo(0, by); ctx.lineTo(w * Math.min(1, p * 1.4), by); ctx.stroke();
  ctx.setLineDash([]);
  if (p > 0.35) {
    ctx.font = 10 * s + "px 'IBM Plex Mono',monospace";
    ctx.globalAlpha = Math.min(1, (p - 0.35) * 3);
    ctx.fillStyle = "#8fb0f5"; ctx.fillText("16 bits", 8 * s, by - 8 * s);
    ctx.fillStyle = "#ef5b6a"; ctx.fillText("padding = 0", 8 * s, by + 16 * s);
    ctx.globalAlpha = 1;
  }
}

function drawWall(c, p, t) {
  const ctx = c.getContext("2d"), w = c.width, h = c.height, s = w / c.clientWidth;
  ctx.clearRect(0, 0, w, h);
  const padB = 20 * s, plotH = h - padB, wallF = 22.05 / 48, wallX = wallF * w;
  const lvl = (f) => (f < wallF ? 0.16 + 0.7 * Math.exp(-f * 2.6) : 0.05);
  const grad = ctx.createLinearGradient(0, plotH, 0, 0);
  grad.addColorStop(0, "rgba(75,130,240,0.55)");
  grad.addColorStop(1, "rgba(123,79,240,0.75)");
  ctx.fillStyle = grad; ctx.beginPath(); ctx.moveTo(0, plotH);
  const xmax = w * Math.min(1, p * 1.15);
  for (let x = 0; x <= xmax; x += 2 * s) {
    const f = x / w, jitter = Math.sin(x * 0.11) * 0.02 + Math.sin(x * 0.031) * 0.03;
    ctx.lineTo(x, plotH - (lvl(f) + (f < wallF ? jitter : jitter * 0.15)) * plotH);
  }
  ctx.lineTo(xmax, plotH); ctx.closePath(); ctx.fill();
  ctx.strokeStyle = "rgba(139,147,167,0.35)"; ctx.lineWidth = s;
  ctx.beginPath(); ctx.moveTo(0, plotH); ctx.lineTo(w, plotH); ctx.stroke();
  ctx.font = 9 * s + "px 'IBM Plex Mono',monospace"; ctx.fillStyle = "#6b7387";
  [0, 12, 24, 36, 48].forEach((f) => { const x = (f / 48) * w; ctx.fillText(f + "k", Math.min(x + 3 * s, w - 22 * s), h - 6 * s); });
  if (p > 0.4) {
    const wp = Math.min(1, (p - 0.4) * 2.2);
    ctx.strokeStyle = "rgba(239,91,106,0.9)"; ctx.lineWidth = 1.5 * s; ctx.setLineDash([6, 5]);
    ctx.beginPath(); ctx.moveTo(wallX, plotH); ctx.lineTo(wallX, plotH * (1 - wp * 0.92)); ctx.stroke();
    ctx.setLineDash([]); ctx.globalAlpha = wp; ctx.fillStyle = "#ef5b6a";
    ctx.fillText("22.05 kHz", wallX + 6 * s, 16 * s); ctx.globalAlpha = 1;
  }
  if (p > 0.7) {
    ctx.globalAlpha = Math.min(1, (p - 0.7) * 3); ctx.fillStyle = "#6b7387";
    ctx.fillText(lang === "fr" ? "vide" : "empty", (wallX + w) / 2 - 12 * s, plotH * 0.5); ctx.globalAlpha = 1;
  }
}

function drawGrid(c, p) {
  const ctx = c.getContext("2d"), w = c.width, h = c.height, s = w / c.clientWidth;
  ctx.clearRect(0, 0, w, h);
  const maxV = Math.pow(5, 4 / 3);
  const yFor = (v) => h - 18 * s - (v / maxV) * (h - 42 * s);
  ctx.font = 9 * s + "px 'IBM Plex Mono',monospace";
  for (let n = 0; n <= 5; n++) {
    const y = yFor(Math.pow(n, 4 / 3));
    ctx.strokeStyle = "rgba(139,147,167,0.16)"; ctx.lineWidth = s;
    ctx.beginPath(); ctx.moveTo(26 * s, y); ctx.lineTo(w - 8 * s, y); ctx.stroke();
    ctx.fillStyle = "#6b7387"; ctx.fillText("n=" + n, 4 * s, y + 3 * s);
  }
  const ease = p * p * (3 - 2 * p), rnd = rng(7);
  for (let i = 0; i < 34; i++) {
    const x = (30 + rnd() * 0.94 * (w / s - 60)) * s;
    const trueV = rnd() * maxV, n = Math.round(Math.pow(trueV, 3 / 4));
    const targetV = Math.pow(Math.max(0, Math.min(5, n)), 4 / 3);
    const v = trueV + (targetV - trueV) * ease, y = yFor(v), snapped = ease > 0.85;
    ctx.beginPath(); ctx.arc(x, y, 3 * s, 0, Math.PI * 2);
    ctx.fillStyle = snapped ? "rgba(160,120,250,0.95)" : "rgba(107,110,180,0.7)"; ctx.fill();
    if (snapped) { ctx.beginPath(); ctx.arc(x, y, 6 * s, 0, Math.PI * 2); ctx.fillStyle = "rgba(123,79,240,0.18)"; ctx.fill(); }
  }
  ctx.fillStyle = "#8fb0f5"; ctx.font = 10 * s + "px 'IBM Plex Mono',monospace";
  ctx.fillText("|X| = n^(4/3)·Δ", w - 108 * s, 14 * s);
  ctx.fillStyle = ease > 0.85 ? "#a078fa" : "#6b7387";
  ctx.fillText("on-grid: " + Math.round(ease * 93) + "%", w - 108 * s, 28 * s);
}

/* ---- conversion: source tree -> mirrored output tree ----
   Scroll-driven like the detection diagrams. The point it has to make in one
   glance is that the *shape* of the folder survives: same nesting, same
   ordering, same non-audio files sitting where they were — only the leaf
   extensions change. So the two trees are drawn side by side with the output
   filling in row by row as `p` advances, rather than as an abstract
   "encoder" box that would say nothing about the layout. */
const CONV_TREE = [
  { depth: 0, name: "Kind of Blue", dir: true },
  { depth: 1, name: "cover.jpg", audio: false },
  { depth: 1, name: "01 So What", audio: true },
  { depth: 1, name: "02 Freddie Freeloader", audio: true },
  { depth: 1, name: "Disc 2", dir: true },
  { depth: 2, name: "03 Blue in Green", audio: true },
];

function drawConv(c, p, ts) {
  const ctx = c.getContext("2d"), w = c.width, h = c.height, s = w / c.clientWidth;
  ctx.clearRect(0, 0, w, h);

  const colW = (w - 30 * s) / 2;
  const leftX = 14 * s, rightX = leftX + colW + 2 * s;
  const top = 40 * s, rowH = (h - top - 22 * s) / CONV_TREE.length;
  const ease = p * p * (3 - 2 * p);

  ctx.font = 10 * s + "px 'IBM Plex Mono',monospace";
  ctx.fillStyle = "#6b7387";
  ctx.fillText(lang === "fr" ? "SOURCE" : "SOURCE", leftX, 20 * s);
  ctx.fillStyle = "#8fb0f5";
  ctx.fillText(lang === "fr" ? "DESTINATION · Opus" : "DESTINATION · Opus", rightX, 20 * s);

  // Divider between the two trees, drawn first so rows sit on top of it.
  ctx.strokeStyle = "rgba(139,147,167,0.18)"; ctx.lineWidth = s;
  ctx.beginPath();
  ctx.moveTo(leftX + colW + s, top - 12 * s);
  ctx.lineTo(leftX + colW + s, h - 12 * s);
  ctx.stroke();

  CONV_TREE.forEach((row, i) => {
    const y = top + i * rowH;
    const indent = row.depth * 11 * s;
    // Each row lands after the one above it. The two numbers are tied
    // together: the last row starts at (n-1)/n · SPREAD, so the fade rate has
    // to be at least 1 / (1 − that) or the bottom of the tree never finishes
    // arriving — which is exactly what a first pass at 0.78 / 1.9 did, leaving
    // the last two rows stuck at 91% and 66% with the scroll already spent.
    // 0.7 / 2.6 clears it with a little slack to spare.
    const SPREAD = 0.7, RATE = 2.6;
    const t = Math.max(0, Math.min(1, (ease - (i / CONV_TREE.length) * SPREAD) * RATE));

    ctx.font = 11 * s + "px 'IBM Plex Mono',monospace";
    ctx.fillStyle = row.dir ? "#c9cede" : "#8b93a7";
    const label = (row.dir ? "▸ " : "") + row.name + (row.audio ? ".flac" : "");
    ctx.fillText(label, leftX + indent, y + 11 * s);

    if (t <= 0) return;

    // The connector, then the mirrored row. Dashes travel left to right so
    // the diagram still reads as *doing* something when the reader stops
    // scrolling — the scroll only decides how many rows exist, not whether
    // anything moves. `ts` is frozen at 0 under reduced motion (see
    // renderFrame's callers), which parks the dashes without a special case.
    // Each row is offset by its own index so the lines don't march in step,
    // which would read as one wide moving band rather than several files.
    ctx.globalAlpha = t * 0.5;
    ctx.strokeStyle = "rgba(123,79,240,0.7)";
    ctx.setLineDash([4 * s, 5 * s]);
    ctx.lineDashOffset = -(((ts || 0) * 0.03 + i * 5) % (9 * s));
    ctx.beginPath();
    ctx.moveTo(leftX + colW - 12 * s, y + 7 * s);
    ctx.lineTo(rightX + indent - 4 * s, y + 7 * s);
    ctx.stroke();
    ctx.setLineDash([]);

    ctx.globalAlpha = t;
    // Converted leaves take the accent; a folder or a copied file stays
    // muted — that contrast is the whole message, so it must not be subtle.
    ctx.fillStyle = row.audio ? "#a078fa" : row.dir ? "#c9cede" : "#6b7387";
    const outLabel = (row.dir ? "▸ " : "") + row.name + (row.audio ? ".opus" : "");
    ctx.fillText(outLabel, rightX + indent, y + 11 * s);
    ctx.globalAlpha = 1;
  });

  ctx.font = 10 * s + "px 'IBM Plex Mono',monospace";
  ctx.fillStyle = "#6b7387";
  ctx.fillText(
    lang === "fr" ? "cover.jpg copiée telle quelle" : "cover.jpg copied verbatim",
    leftX,
    h - 5 * s,
  );
}

/* ---- realistic streaming spectrogram (inferno colormap) ---- */
const INFERNO = [
  [4, 4, 18], [22, 20, 74], [58, 30, 120], [120, 34, 120],
  [186, 44, 92], [226, 66, 54], [244, 120, 30], [250, 188, 52],
  [252, 255, 190],
];
function inferno(t) {
  t = t < 0 ? 0 : t > 1 ? 1 : t;
  const seg = t * (INFERNO.length - 1), i = Math.floor(seg), fr = seg - i;
  const a = INFERNO[i], b = INFERNO[Math.min(i + 1, INFERNO.length - 1)];
  return [a[0] + (b[0] - a[0]) * fr, a[1] + (b[1] - a[1]) * fr, a[2] + (b[2] - a[2]) * fr];
}
function hsh(x, y) { const v = Math.sin(x * 127.1 + y * 311.7) * 43758.5453; return v - Math.floor(v); }
function vnoise(x, y) {
  const xi = Math.floor(x), yi = Math.floor(y), xf = x - xi, yf = y - yi;
  const u = xf * xf * (3 - 2 * xf), v = yf * yf * (3 - 2 * yf);
  const a = hsh(xi, yi), b = hsh(xi + 1, yi), c = hsh(xi, yi + 1), d = hsh(xi + 1, yi + 1);
  return a + (b - a) * u + (c - a) * v + (a - b - c + d) * u * v;
}
function fbm(x, y) { return 0.6 * vnoise(x, y) + 0.3 * vnoise(x * 2.3, y * 2.3) + 0.1 * vnoise(x * 4.7, y * 4.7); }

const SPEC_W = 560, SPEC_H = 240, SPEC_WALL = 22.05 / 48;
let specBuf = null, specBctx = null, specImg = null, specGx = 0, specLast = 0;

// energy at time-column gx and frequency fraction f (0 = low/bottom, 1 = high/top).
// Structure is time-driven (vertical striations) and aperiodic (noise, not sines)
// so it reads like a real spectrogram rather than a repeating pattern.
function specField(gx, f) {
  const tilt = Math.exp(-f * 2.6) * 0.92 + 0.05;             // spectral tilt: bright lows
  const slow = fbm(gx * 0.03, 7.3);                          // song loudness envelope
  const colN = fbm(gx * 0.55, 2.1);                          // per-column variation
  const colGain = 0.30 + 0.55 * slow + 0.35 * colN;
  const os = fbm(gx * 0.9, 13.0);
  const onset = Math.pow(Math.max(0, os - 0.62) / 0.38, 2) * 1.3;  // bright vertical streaks
  const grain = fbm(gx * 0.8, f * 22) - 0.5;                 // fine sharp detail
  let e = tilt * colGain * (0.9 + 0.5 * grain) + onset * tilt * 0.9;
  e += 0.04;
  if (f < 0.28) e += 0.10 * Math.max(0, fbm(gx * 1.3, f * 30) - 0.55);  // low-band sparkle
  if (f > SPEC_WALL) e = 0.02 + 0.03 * Math.max(0, os - 0.5);          // near-empty above cut-off
  return e;
}
function specColumn(px, gx) {
  const d = specImg.data;
  for (let py = 0; py < SPEC_H; py++) {
    const f = (SPEC_H - 1 - py) / (SPEC_H - 1);
    const e = specField(gx, f);
    const col = inferno(Math.pow(e < 0 ? 0 : e > 1 ? 1 : e, 0.60));
    const idx = (py * SPEC_W + px) * 4;
    d[idx] = col[0]; d[idx + 1] = col[1]; d[idx + 2] = col[2]; d[idx + 3] = 255;
  }
}
function specInit() {
  specBuf = document.createElement("canvas");
  specBuf.width = SPEC_W; specBuf.height = SPEC_H;
  specBctx = specBuf.getContext("2d");
  specImg = specBctx.createImageData(SPEC_W, SPEC_H);
  for (let x = 0; x < SPEC_W; x++) specColumn(x, x);
  specGx = SPEC_W;
  specBctx.putImageData(specImg, 0, 0);
}
function specScroll(step) {
  const d = specImg.data, rowBytes = SPEC_W * 4;
  for (let n = 0; n < step; n++) {
    for (let py = 0; py < SPEC_H; py++) {
      const rs = py * rowBytes;
      d.copyWithin(rs, rs + 4, rs + rowBytes);
    }
    specColumn(SPEC_W - 1, specGx++);
  }
  specBctx.putImageData(specImg, 0, 0);
}

// Animated, realistic spectrogram: an inferno-mapped energy field that streams
// left over time (like a live analyzer). Bright yellows at the bottom (low freq),
// fading to red/purple/black going up, with note onsets as vertical streaks and a
// sharp cut-off wall at 22.05 kHz - the visual tell of an upsampled file.
function drawSpec(c, ts) {
  const ctx = c.getContext("2d"), w = c.width, h = c.height, s = w / c.clientWidth;
  if (!specBuf) specInit();
  if (ts - specLast > 55) { specScroll(1); specLast = ts; }
  const padL = 34 * s, top = 8 * s, plotW = w - padL - 8 * s, plotH = h - 18 * s - 8 * s;
  ctx.clearRect(0, 0, w, h);
  ctx.imageSmoothingEnabled = false;   // keep vertical striations crisp (like ffmpeg)
  ctx.drawImage(specBuf, padL, top, plotW, plotH);
  const wallY = top + (1 - SPEC_WALL) * plotH;
  ctx.strokeStyle = "rgba(255,255,255,0.72)"; ctx.lineWidth = 1.1 * s; ctx.setLineDash([6, 5]);
  ctx.beginPath(); ctx.moveTo(padL, wallY); ctx.lineTo(w - 8 * s, wallY); ctx.stroke(); ctx.setLineDash([]);
  ctx.font = 9 * s + "px 'IBM Plex Mono',monospace"; ctx.fillStyle = "#8a90a6";
  ctx.fillText("48 kHz", 2 * s, 14 * s);
  ctx.fillText("24 kHz", 2 * s, top + plotH * 0.5 + 3 * s);
  ctx.fillText("0", 2 * s, top + plotH);
  ctx.fillStyle = "#fff";
  ctx.fillText((lang === "fr" ? "22,05 kHz" : "22.05 kHz") + " — cut-off", padL + 6 * s, wallY - 5 * s);
  ctx.fillStyle = "#c9cede"; ctx.fillText("96 kHz · 24 bit · stereo · FLAC", padL, h - 5 * s);
}

// Draw one frame at time `ts`. Split out from the loop so it can also be called
// on demand (reduced-motion mode redraws on scroll/resize instead of looping).
function renderFrame(ts) {
  try {
    const dpr = Math.min(window.devicePixelRatio || 1, 1.5);
    const k = INTENSITY / 6, t = T();
    const bg = document.getElementById("bgc");
    if (bg) { sizeCanvas(bg, dpr); drawBg(bg, ts, k); }
    const gUp = document.getElementById("gUp");
    if (gUp && visible(gUp)) { sizeCanvas(gUp, dpr); drawUp(gUp, progress(gUp), t); }
    const gWall = document.getElementById("gWall");
    if (gWall && visible(gWall)) { sizeCanvas(gWall, dpr); drawWall(gWall, progress(gWall), t); }
    const gGrid = document.getElementById("gGrid");
    if (gGrid && visible(gGrid)) { sizeCanvas(gGrid, dpr); drawGrid(gGrid, progress(gGrid)); }
    const gConv = document.getElementById("gConv");
    if (gConv && visible(gConv)) {
      sizeCanvas(gConv, dpr);
      drawConv(gConv, progress(gConv, CONV_START_VH, CONV_END_VH), ts);
    }
    const gSpec = document.getElementById("gSpec");
    if (gSpec && visible(gSpec)) { sizeCanvas(gSpec, dpr); drawSpec(gSpec, ts); }
  } catch (e) {
    /* never let a bad frame kill the animation loop */
  }
}

// Respect the user's "reduce motion" OS setting: no continuous animation loop.
// The graphics still render (a single static frame, frozen time), and the
// scroll-driven detection diagrams still update — but only in response to the
// user's own scrolling/resizing, never on their own. drawBg and drawSpec are
// called with a fixed timestamp so the background bars and the spectrogram do
// not stream. Re-checked live via matchMedia so toggling the OS setting takes
// effect without a reload.
const motionQuery = window.matchMedia("(prefers-reduced-motion: reduce)");

function frame(ts) {
  renderFrame(ts);
  if (!motionQuery.matches) requestAnimationFrame(frame);
}

let staticScheduled = false;
function renderStaticFrame() {
  // Coalesce bursts of scroll events into one draw per animation frame.
  if (staticScheduled) return;
  staticScheduled = true;
  requestAnimationFrame(() => {
    staticScheduled = false;
    renderFrame(0);
  });
}

function startMotion() {
  if (motionQuery.matches) {
    renderFrame(0);
    window.addEventListener("scroll", renderStaticFrame, { passive: true });
  } else {
    requestAnimationFrame(frame);
  }
}

// Scroll reveal. The hiding class is only ever applied here, and only when
// animations are allowed and the browser can tell us when a section arrives —
// so no JS, no IntersectionObserver, or reduced-motion all leave the page
// fully visible rather than blank. `once: true` in spirit: each section is
// unobserved as soon as it has been seen, so scrolling back up doesn't
// re-play anything (a section fading out again while you scroll away is the
// most common way this pattern turns annoying).
function setupReveal() {
  if (motionQuery.matches || !("IntersectionObserver" in window)) return;
  const sections = document.querySelectorAll("main .section");
  const io = new IntersectionObserver(
    (entries) => {
      for (const e of entries) {
        if (!e.isIntersecting) continue;
        e.target.classList.add("seen");
        io.unobserve(e.target);
      }
    },
    // A slight bottom inset means a section starts arriving just before it is
    // fully on screen, rather than after the reader is already looking at it.
    { rootMargin: "0px 0px -12% 0px", threshold: 0.05 },
  );
  for (const el of sections) {
    el.classList.add("reveal");
    io.observe(el);
  }
}
setupReveal();

window.addEventListener("resize", () => {
  specState.key = "";
  if (motionQuery.matches) renderStaticFrame();
});
// If the OS setting changes while the page is open, switch modes live.
motionQuery.addEventListener("change", () => {
  window.removeEventListener("scroll", renderStaticFrame);
  startMotion();
});
startMotion();
