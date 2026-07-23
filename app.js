/* FlacCompagnon landing — vanilla i18n, interactive tooltips and canvas
   animations. Ported from the design handoff (no framework). */

const DICT = {
  fr: {
    nPrev: "Aperçu", nDet: "Détections", nInt: "Intégrité", nSpec: "Spectrogrammes", nExp: "Export", nPipe: "Pipeline", nDl: "Télécharger", nDoc: "Docs",
    kick: "Open source · Rust + Tauri · macOS · Windows · Linux",
    h1a: "Votre lossless est-il", h1b: "vraiment lossless ?",
    sub: "FlacCompagnon analyse vos fichiers audio et démasque les faux FLAC : upscaling, upsampling et transcodage — y compris les transcodes AAC, à tous les débits. Déposez un dossier, l'analyse fait le reste.",
    ctaDl: "Télécharger", ctaGh: "Voir sur GitHub",
    note: "Gratuit · MIT · Vos fichiers ne sont jamais modifiés",
    s1k: "Aperçu", s1t: "L'analyse en un coup d'œil",
    s1p: "Déposez un dossier ou des fichiers : chaque piste est décodée et passée au crible en parallèle. Un tag coloré par détection — et au survol, l'explication du verdict.",
    cap1: "↑ Aperçu interactif de l'interface (thème sombre) — survolez les tags de détection.",
    thFile: "Fichier", thFmt: "Format", thDur: "Durée", thSr: "Échantillonnage", thBits: "Bits réels", thCut: "Cut-off", thDet: "Détections",
    sumCount: "4 fichiers", chClean: "1 clean", chUps: "1 upsamplé", chTr: "1 transcodé", chSus: "1 suspect",
    tipClean: "Aucune détection : profondeur effective conforme, spectre plein jusqu'à la fréquence de Nyquist (la moitié de l'échantillonnage), pas de signature lossy.",
    tipSus: "Roll-off doux du spectre sans falaise nette : peut être un transcodage… ou un enregistrement naturellement sombre. Vérifiez le spectrogramme.",
    s2k: "Détections", s2t: "Trois détections indépendantes",
    s2p: "Le modèle du Lossless Audio Checker, réimplémenté de zéro et validé sur des transcodes réels. Trois signatures indépendantes, chacune ciblant une manière différente de fabriquer un faux lossless.",
    s2note: "Ces détections sont des heuristiques éclairées, pas une preuve cryptographique — les enregistrements naturellement « sombres » (acoustique, classique, bandes analogiques) peuvent déclencher un faux positif doux. Le spectrogramme reste l'arbitre final. La grille AAC, elle, frôle la preuve : la musique authentique ne retombe jamais dessus.",
    d1t: "Upscaling — fausse résolution",
    d1p: "FlacCompagnon regarde combien de bits chaque échantillon utilise réellement. Si les bits de poids faible restent toujours à zéro, ils ne portent aucune information : un fichier annoncé en 24 bits qui n'en utilise que 16 est un 16 bits « gonflé ».",
    d1tip: "Marqué Upscaled : profondeur déclarée 24 bits, profondeur effective ≤ 16 bits. Les 8 bits de poids faible ne portent aucune information réelle.",
    d1cap: "24 bits déclarés — les 8 bits bas sont vides",
    d2t: "Upsampling — faux échantillonnage",
    d2p: "Une analyse fréquentielle moyennée sur toute la piste. Si un conteneur « hi-res » 96 kHz s'arrête net vers 22 kHz, toute la bande supplémentaire est vide — le fichier a été suréchantillonné depuis un CD.",
    d2tip: "Marqué Upsampled : conteneur 96 kHz, mais le contenu s'arrête net à 22,05 kHz — le mur caractéristique d'un suréchantillonnage depuis 44,1 kHz.",
    d2cap: "Le mur du cut-off à 22,05 kHz",
    d3t: "Transcoding — source lossy",
    d3p: "La signature la plus forte. FlacCompagnon rejoue le calcul interne de l'encodeur AAC : si le fichier est un transcode, ses coefficients « retombent » exactement sur la grille de valeurs que seul l'AAC produit, au bon alignement parmi 1024 essais. Ça démasque l'AAC à tous les débits, même 256 kbps.",
    d3tip: "Marqué Transcoded : 70–97 % des bandes retombent sur la grille de quantification AAC au meilleur alignement — la musique authentique ne dépasse jamais 1,4 %.",
    d3cap: "Les coefficients « snappent » sur la grille AAC",
    s3k: "Intégrité", s3t: "Au-delà des détections",
    s3p: "Six contrôles et fonctions complètent l'analyse.",
    i1t: "Vérification MD5 FLAC",
    i1p: "Quand elle est présente, un FLAC contient l'empreinte MD5 de son audio d'origine. FlacCompagnon décode entièrement le fichier et recompare le hash — le même contrôle que <code>flac -t</code>, fusionné avec l'analyse en un seul passage. Les FLAC sans empreinte sont signalés comme tels.",
    i2t: "Fake stereo",
    i2p: "Deux canaux strictement identiques ? Le fichier est en réalité du dual-mono déguisé en stéréo.",
    i3t: "Clipping",
    i3p: "Compte les salves d'échantillons à pleine échelle et mesure aussi le vrai pic (dBTP) par suréchantillonnage ×4 : certains masters écrêtent « entre » les échantillons, là où le pic classique ne voit rien — le signe d'un master trop fort, indépendamment du caractère lossless.",
    i4t: "Détection du format réel",
    i4p: "FlacCompagnon lit les octets d'en-tête, pas l'extension. Un <code>.flac</code> qui est en réalité un WAV — ou l'inverse — est repéré et signalé en rouge.",
    i5t: "Dynamique (DR)",
    i5p: "FlacCompagnon mesure la plage dynamique de chaque piste : l'écart entre les crêtes et le niveau moyen des passages forts. Une valeur élevée signale un master respectueux de la dynamique — typique des éditions Full Dynamic Range. Une valeur faible trahit un master écrasé par la « loudness war ».",
    i6t: "Hi-Res & DSD",
    i6p: "Les vraies pistes haute résolution et les fichiers DSD (<code>.dsf</code> / <code>.dff</code>) reçoivent un badge vérifié — et FlacCompagnon distingue le DSD authentique d'un PCM converti après coup (nécessite ffmpeg ; sans lui, seul l'en-tête est vérifié).",
    s4k: "Spectrogrammes", s4t: "Le juge de paix",
    s4p: "Un clic génère un spectrogramme haute résolution par piste, avec axe de fréquences gradué jusqu'à la fréquence de Nyquist (la moitié de l'échantillonnage, soit 22,05 kHz pour un CD à 44,1 kHz). Le cut-off se voit à l'œil nu.",
    s4n1: "Rendu via ffmpeg, détecté automatiquement sur votre système.",
    s4n2: "Un dossier <code>spectres/</code> est créé dans chaque dossier contenant des fichiers audio, et un PNG est généré par piste — uniquement si vous cliquez sur le bouton « Generate spectrograms ».",
    s5k: "Pipeline", s5t: "Comment ça marche",
    s5p: "Glissez-déposez un dossier ou des fichiers — ou choisissez-les avec le bouton d'import. Chaque piste est décodée, puis passée en revue par une batterie d'analyses, et le verdict s'affiche.",
    p1: "dossier déposé", p2: "décodage · symphonia", p3: "analyse streaming",
    p3a: "FFT ▸ cut-off", p3b: "MDCT ▸ grille AAC", p3c: "bits effectifs", p3d: "clipping · fake stereo",
    p4: "table de résultats",
    s5note: "Les FLAC sont décodés une seule fois : le même passage nourrit l'analyse et le hash MD5. Les fichiers sont traités en parallèle — un worker par cœur CPU, moins un pour garder l'interface fluide. Le décodage s'appuie sur Symphonia, une bibliothèque de décodage audio 100 % Rust (FLAC, WAV, ALAC, AAC…).",
    s6k: "Téléchargement", s6t: "Une petite app native",
    s6p: "Installeurs signés par la CI pour les trois plateformes, publiés sur les releases GitHub.",
    dlBtn: "Télécharger",
    ffnote: "ffmpeg n'est requis que pour les spectrogrammes et pour l'analyse de contenu des fichiers DSD — le reste de l'analyse s'en passe entièrement.",
    ffinstall: "Pas de ffmpeg installé ? Consultez le <a href=\"https://github.com/craft-and-code/FlacCompagnon#prerequisites\" target=\"_blank\" rel=\"noopener\">README sur GitHub</a> pour l'installer sur macOS, Windows ou Linux.",
    s7k: "Export", s7t: "Emportez vos résultats",
    s7p: "Un clic sur <strong>Save…</strong> écrit deux fichiers, même nom, même dossier : un <code>.csv</code> — une ligne par piste, toutes les colonnes de l'analyse, prêt pour un tableur — et un <code>.json</code> qui conserve l'analyse complète, y compris le détail de chaque détection. Le nom et l'emplacement proposés reprennent le dossier analysé.",
    s7n2: "Déposez un <code>.json</code> déjà exporté sur la fenêtre pour recharger le tableau tel quel — aucun fichier audio n'est redécodé. Pas de bouton dédié : c'est le même geste que déposer un dossier.",
    s7n1: "Astuce : la corbeille en bout de ligne écarte un fichier de la liste — il ne figurera dans aucun des deux fichiers exportés.",
    refT: "Références",
    ref1: "J. Lacroix, Y. Prime, A. Remy & O. Derrien — « Lossless Audio Checker: A Software for the Detection of Upscaling, Upsampling, and Transcoding in Lossless Musical Tracks », AES 139th Convention, Paper 9416, New York, 2015.",
    ref2: "O. Derrien et al. — « Detection of Genuine Lossless Audio Files: Application to the MPEG-AAC Codec », Journal of the AES, 2019.",
    ref3: "FlacCompagnon est un successeur open source, réécrit de zéro, du Lossless Audio Checker (discontinué).",
    ftLeft: "© 2026 FlacCompagnon · Licence MIT · Construit en Rust + Tauri",
  },
  en: {
    nPrev: "Preview", nDet: "Detections", nInt: "Integrity", nSpec: "Spectrogrammes", nExp: "Export", nPipe: "Pipeline", nDl: "Download", nDoc: "Docs",
    kick: "Open source · Rust + Tauri · macOS · Windows · Linux",
    h1a: "Is your lossless", h1b: "actually lossless?",
    sub: "FlacCompagnon analyzes your audio files and exposes fake FLACs: upscaling, upsampling and transcoding — including AAC transcodes at every bitrate. Drop a folder, the analysis does the rest.",
    ctaDl: "Download", ctaGh: "View on GitHub",
    note: "Free · MIT · Your files are never modified",
    s1k: "Preview", s1t: "The analysis at a glance",
    s1p: "Drop a folder or files: every track is decoded and screened in parallel. One coloured tag per detection — hover it for the reasoning behind the verdict.",
    cap1: "↑ Interactive preview of the interface (dark theme) — hover the detection tags.",
    thFile: "File", thFmt: "Format", thDur: "Length", thSr: "Sample rate", thBits: "Real bits", thCut: "Cut-off", thDet: "Detections",
    sumCount: "4 files", chClean: "1 clean", chUps: "1 upsampled", chTr: "1 transcoded", chSus: "1 suspected",
    tipClean: "No detection: effective bit depth matches, spectrum full up to the Nyquist frequency (half the sample rate), no lossy signature.",
    tipSus: "Gentle spectral roll-off with no sharp cliff: could be a transcode… or a naturally dark recording. Check the spectrogram.",
    s2k: "Detections", s2t: "Three independent detections",
    s2p: "The Lossless Audio Checker model, re-implemented from scratch and validated on real transcodes. Three independent signatures, each targeting a different way of faking lossless.",
    s2note: "These detections are informed heuristics, not cryptographic proof — naturally “dark” recordings (acoustic, classical, analog tape) can trip a soft false positive. The spectrogram is the final arbiter. The AAC grid, however, is close to proof: genuine music never lands on it.",
    d1t: "Upscaling — fake resolution",
    d1p: "FlacCompagnon checks how many bits each sample actually uses. If the low bits are always zero, they carry no information: a file advertised as 24-bit that really uses only 16 is an inflated 16-bit signal.",
    d1tip: "Flagged Upscaled: declared depth 24-bit, effective depth ≤ 16-bit. The 8 low bits carry no real information.",
    d1cap: "Declared 24-bit — the 8 low bits are empty",
    d2t: "Upsampling — fake sample rate",
    d2p: "A frequency analysis averaged over the whole track. If a “hi-res” 96 kHz container stops dead around 22 kHz, all the extra bandwidth is empty — the file was upsampled from a CD.",
    d2tip: "Flagged Upsampled: 96 kHz container, but the content stops dead at 22.05 kHz — the tell-tale wall of upsampling from 44.1 kHz.",
    d2cap: "The cut-off wall at 22.05 kHz",
    d3t: "Transcoding — lossy source",
    d3p: "The strongest signature. FlacCompagnon replays the AAC encoder's own internal computation: if the file is a transcode, its coefficients land back exactly on the grid of values only AAC produces, at the right alignment out of 1024 tries. It exposes AAC at every bitrate, even 256 kbps.",
    d3tip: "Flagged Transcoded: 70–97% of bands land on AAC's quantization grid at the best alignment — genuine music never exceeds 1.4%.",
    d3cap: "Coefficients snapping onto the AAC grid",
    s3k: "Integrity", s3t: "Beyond the detections",
    s3p: "Six checks and features round out the analysis.",
    i1t: "FLAC MD5 verification",
    i1p: "Every FLAC stores the MD5 of its decoded audio. FlacCompagnon fully decodes the file and recomputes the hash — the same check as flac -t, fused into the analysis pass.",
    i2t: "Fake stereo",
    i2p: "Both channels strictly identical? The file is really dual-mono dressed up as stereo.",
    i3t: "Clipping",
    i3p: "Counts full-scale sample runs and also measures the true peak (dBTP) via 4x oversampling: some masters clip \"between\" the samples, where the classic peak sees nothing — the sign of an over-loud master, independent of losslessness.",
    i4t: "Real format detection",
    i4p: "FlacCompagnon reads the header bytes, not the extension. A <code>.flac</code> that is really a WAV — or the reverse — is caught and flagged in red.",
    i5t: "Dynamics (DR)",
    i5p: "FlacCompagnon measures each track's dynamic range: the gap between the peaks and the average level of the loud passages. A high value signals a master that preserves dynamics — typical of Full Dynamic Range editions — while a low value betrays a loudness-war master.",
    i6t: "Hi-Res & DSD",
    i6p: "Genuine high-resolution tracks and DSD files (<code>.dsf</code> / <code>.dff</code>) earn a verified badge — and FlacCompagnon tells real DSD from PCM converted after the fact (requires ffmpeg; without it, only the header is verified).",
    s4k: "Spectrograms", s4t: "The final arbiter",
    s4p: "One click renders a high-resolution spectrogram per track, with a labelled frequency axis up to the Nyquist frequency (half the sample rate, e.g. 22.05 kHz for a 44.1 kHz CD). The cut-off is visible to the naked eye.",
    s4n1: "Rendered via ffmpeg, located automatically on your system.",
    s4n2: "A <code>spectres/</code> folder is created inside each folder that holds audio files, and one PNG per track is generated — only when you click the “Generate spectrograms” button.",
    s5k: "Pipeline", s5t: "How it works",
    s5p: "Drag and drop a folder or files — or pick them with the import button. Each track is decoded, then run through a battery of analyses, and the verdict appears.",
    p1: "dropped folder", p2: "decode · symphonia", p3: "streaming analyzer",
    p3a: "FFT ▸ cut-off", p3b: "MDCT ▸ AAC grid", p3c: "effective bits", p3d: "clipping · fake stereo",
    p4: "results table",
    s5note: "FLAC files are decoded once: the same pass feeds the analysis and the MD5 hash. Files are processed in parallel — one worker per CPU core, minus one to keep the UI responsive. Decoding relies on Symphonia, a pure-Rust audio decoding library (FLAC, WAV, ALAC, AAC…).",
    s6k: "Download", s6t: "A small native app",
    s6p: "CI-built installers for all three platforms, published on the GitHub releases.",
    dlBtn: "Download",
    ffnote: "ffmpeg is only needed for spectrograms and for DSD content analysis — the rest of the analysis works entirely without it.",
    ffinstall: "No ffmpeg installed? See the <a href=\"https://github.com/craft-and-code/FlacCompagnon#prerequisites\" target=\"_blank\" rel=\"noopener\">README on GitHub</a> to install it on macOS, Windows or Linux.",
    s7k: "Export", s7t: "Take your results with you",
    s7p: "Clicking <strong>Save…</strong> writes two files, same name, same folder: a <code>.csv</code> — one row per track, every analysis column, ready for a spreadsheet — and a <code>.json</code> that keeps the full analysis, including each detection's detail. The suggested name and location follow the analyzed folder.",
    s7n2: "Drop a previously-exported <code>.json</code> onto the window to reload the table as-is — no audio is re-decoded. No dedicated button: same gesture as dropping a folder.",
    s7n1: "Tip: the trash icon at the end of a row removes a file from the list — it will not appear in either exported file.",
    refT: "References",
    ref1: "J. Lacroix, Y. Prime, A. Remy & O. Derrien — “Lossless Audio Checker: A Software for the Detection of Upscaling, Upsampling, and Transcoding in Lossless Musical Tracks”, AES 139th Convention, Paper 9416, New York, 2015.",
    ref2: "O. Derrien et al. — “Detection of Genuine Lossless Audio Files: Application to the MPEG-AAC Codec”, Journal of the AES, 2019.",
    ref3: "FlacCompagnon is an open-source, from-scratch successor to the discontinued Lossless Audio Checker.",
    ftLeft: "© 2026 FlacCompagnon · MIT License · Built with Rust + Tauri",
  },
};

let lang = "fr";
const $ = (s) => document.querySelector(s);
const specState = { key: "" };

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
// Default to the browser language: French UI only if the browser is French,
// English everywhere else. A previous manual choice (localStorage) wins.
let initial = (navigator.language || "").toLowerCase().startsWith("fr") ? "fr" : "en";
try {
  const saved = localStorage.getItem("fc-lang");
  if (saved === "fr" || saved === "en") initial = saved;
} catch {}
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
function progress(c) {
  const r = c.getBoundingClientRect(), vh = window.innerHeight;
  return Math.max(0, Math.min(1, (vh * 0.88 - r.top) / (vh * 0.62)));
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
