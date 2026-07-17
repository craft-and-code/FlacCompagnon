import "./styles.css";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { open, save } from "@tauri-apps/plugin-dialog";

import type { FolderReport, Progress, Theme } from "./types";
import * as api from "./api";
import {
  commonDir,
  csvNameFrom,
  deleteBtn,
  detectionsTd,
  drCell,
  escapeHtml,
  fmtCutoff,
  fmtDuration,
  md5Cell,
  revealBtn,
  truePeakCell,
} from "./format";

// --- DOM handles ------------------------------------------------------------

const $ = <T extends HTMLElement>(id: string) =>
  document.getElementById(id) as T;
const dropzone = $("dropzone");
const pickBtn = $<HTMLButtonElement>("pick-btn");
const spectroBtn = $<HTMLButtonElement>("spectro-btn");
const saveBtn = $<HTMLButtonElement>("save-btn");
const resetBtn = $<HTMLButtonElement>("reset-btn");
const themeBtn = $<HTMLButtonElement>("theme-btn");
const cancelBtn = $<HTMLButtonElement>("cancel-btn");
const dropGuard = $("drop-guard");
const progressEl = $("progress");
const progressBar = $("progress-bar");
const progressText = $("progress-text");
const resultsEl = $("results");
const summaryEl = $("summary");
const rootPathEl = $("root-path");
const table = $<HTMLTableElement>("results-table");
const toast = $("toast");

// --- State ------------------------------------------------------------------

let currentTargets: string[] = []; // dropped/selected files or folders
let lastReport: FolderReport | null = null;
let busy = false;
let ffmpegAvailable = false;
let userCancelled = false;

// --- Small UI helpers -------------------------------------------------------

function showToast(msg: string, kind: "info" | "error" = "info") {
  toast.textContent = msg;
  toast.className = `toast ${kind}`;
  window.setTimeout(() => toast.classList.add("hidden"), 4200);
}

function updateControls() {
  pickBtn.disabled = busy;
  saveBtn.disabled = busy || !lastReport;
  spectroBtn.disabled = busy || currentTargets.length === 0 || !ffmpegAvailable;
  spectroBtn.title = ffmpegAvailable
    ? ""
    : "ffmpeg was not found on your system — install it to enable spectrograms";
  resetBtn.classList.toggle("hidden", busy || !lastReport);
}

function setBusy(on: boolean, label = "Working…", keepResults = false) {
  busy = on;
  dropGuard.classList.add("hidden");
  if (on) {
    userCancelled = false;
    cancelBtn.disabled = false;
    dropzone.classList.add("hidden");
    if (!keepResults) resultsEl.classList.add("hidden");
    progressEl.classList.remove("hidden");
    progressText.textContent = label;
    progressBar.style.width = "0%";
  } else {
    progressEl.classList.add("hidden");
  }
  updateControls();
}

function showDropScreen() {
  lastReport = null;
  currentTargets = [];
  resultsEl.classList.add("hidden");
  progressEl.classList.add("hidden");
  dropzone.classList.remove("hidden");
}

function reset() {
  if (busy) return;
  showDropScreen();
  updateControls();
}

async function cancelTask() {
  if (!busy) return;
  userCancelled = true;
  cancelBtn.disabled = true;
  progressText.textContent = "Cancelling…";
  try {
    await api.cancelTask();
  } catch {
    /* ignore */
  }
}

// Full default path for the Save dialog: <common folder>/<name>.csv, so it
// opens in the same location shown above the table. The chosen stem also
// names the JSON sibling written alongside it (save always writes both).
function defaultSavePath(): string {
  const nameSource =
    currentTargets.length === 1 ? currentTargets[0] : (lastReport?.root ?? "");
  const name = csvNameFrom(nameSource);
  const dir = lastReport ? commonDir(lastReport.files.map((f) => f.path)) : "";
  if (!dir) return name;
  const sep = dir.includes("\\") ? "\\" : "/";
  return dir.endsWith(sep) ? `${dir}${name}` : `${dir}${sep}${name}`;
}

function updateProgress(p: Progress, verb: string) {
  const pct = p.total > 0 ? Math.round((p.current / p.total) * 100) : 0;
  progressBar.style.width = `${pct}%`;
  progressText.textContent = p.file
    ? `${verb} ${p.current + 1}/${p.total} — ${p.file}`
    : `${verb} ${p.total}/${p.total}`;
}

// --- Rendering --------------------------------------------------------------

function renderReport(report: FolderReport) {
  lastReport = report;
  // Show the common folder of all listed files; recomputed as files are added.
  rootPathEl.textContent = commonDir(report.files.map((f) => f.path));
  dropzone.classList.add("hidden");

  let cClean = 0;
  let cUpscaled = 0;
  let cUpsampled = 0;
  let cTranscoded = 0;
  let cSuspicious = 0;
  let md5Bad = 0;
  let md5Missing = 0;
  for (const f of report.files) {
    const d = f.detections;
    if (d.summary === "Clean") cClean++;
    if (d.summary === "Suspicious") cSuspicious++;
    if (d.upscaling) cUpscaled++;
    if (d.upsampling) cUpsampled++;
    if (d.transcoding === "detected") cTranscoded++;
    if (f.flac_md5?.state === "Mismatch") md5Bad++;
    if (f.flac_md5?.state === "NoSignature") md5Missing++;
  }

  const chips = [
    cClean ? `<span class="chip v-clean">${cClean} clean</span>` : "",
    cUpscaled ? `<span class="chip v-transcoded">${cUpscaled} upscaled</span>` : "",
    cUpsampled ? `<span class="chip v-upsampled">${cUpsampled} upsampled</span>` : "",
    cTranscoded ? `<span class="chip v-transcoded">${cTranscoded} transcoded</span>` : "",
    cSuspicious ? `<span class="chip v-suspicious">${cSuspicious} suspicious</span>` : "",
    report.has_flac && md5Bad ? `<span class="chip v-transcoded">${md5Bad} MD5 mismatch</span>` : "",
    report.has_flac && md5Missing ? `<span class="chip v-suspicious">${md5Missing} no MD5</span>` : "",
  ]
    .filter(Boolean)
    .join(" ");
  summaryEl.innerHTML = `<span class="count">${report.files.length} files</span> ${chips}`;

  const showMd5 = report.has_flac;
  // Quality column only appears when at least one file earned a badge.
  const showBadge = report.files.some((f) => f.badge != null);
  const headers = [
    "", // reveal button
    "File",
    "Format",
    ...(showBadge ? ["Quality"] : []),
    "Rate",
    "Bits",
    "Real bits",
    "Length",
    "Detections",
    "Cutoff",
    "Ch",
    "Stereo",
    "Clipping",
    "True Peak",
    "Dynamics",
    ...(showMd5 ? ["MD5"] : []),
    "", // delete button
  ];
  const thead = table.tHead ?? table.createTHead();
  thead.innerHTML = `<tr>${headers.map((h) => `<th>${h}</th>`).join("")}</tr>`;

  const tbody = table.tBodies[0] ?? table.createTBody();
  tbody.innerHTML = report.files
    .map((f) => rowHtml(f, headers.length, showMd5, showBadge))
    .join("");

  resultsEl.classList.remove("hidden");
  updateControls();
}

function rowHtml(
  f: FolderReport["files"][number],
  nCols: number,
  showMd5: boolean,
  showBadge: boolean,
): string {
  const fname = `<td class="fname has-tip" title="${escapeHtml(f.path)}">${escapeHtml(f.file_name)}</td>`;

  // Verified quality badge cell (custom chip — the official DSD / Hi-Res Audio
  // logos are trademarked). Granted only when no detection contradicts it.
  let badgeCell = "";
  if (showBadge) {
    if (f.badge) {
      const unverified = f.badge.includes("unverified");
      const dsdSource = f.badge.includes("DSD source");
      const tip = unverified
        ? "Container header is authentic, but the content could not be analyzed (ffmpeg not found)."
        : dsdSource
          ? "Hi-Res PCM carrying the sigma-delta noise signature of a DSD master — verified by analysis."
          : "Verified by analysis: the claimed quality is not contradicted by any detection.";
      const label = f.badge.replace(" (unverified)", "?").replace(" (DSD source)", "·DSD");
      badgeCell = `<td><span class="qbadge${unverified ? " q-unk" : ""} has-tip" title="${tip}">${escapeHtml(label)}</span></td>`;
    } else {
      badgeCell = `<td class="c-muted">—</td>`;
    }
  }

  if (f.error) {
    const span = nCols - 3; // reveal + file precede, delete trails
    return `<tr><td class="reveal">${revealBtn(f.path)}</td>${fname}<td colspan="${span}" class="c-bad">Error: ${escapeHtml(f.error)}</td><td class="rowdel">${deleteBtn(f.path)}</td></tr>`;
  }

  const bits = f.declared_bits != null ? `${f.declared_bits}-bit` : "float";

  // Real bit depth: green if it matches the declared depth, red (with an
  // explanation on hover) when the content uses fewer bits than declared.
  let realCell: string;
  if (f.real_bit_depth == null) {
    realCell = `<td class="c-muted">—</td>`;
  } else if (f.declared_bits != null && f.real_bit_depth < f.declared_bits) {
    realCell = `<td class="c-bad has-tip" title="Only ${f.real_bit_depth} of the declared ${f.declared_bits} bits carry real information — the low bits are always zero (the content was upscaled to a higher bit depth).">${f.real_bit_depth}-bit</td>`;
  } else {
    realCell = `<td class="c-ok">${f.real_bit_depth}-bit</td>`;
  }

  let stereo: string;
  if (f.fake_stereo == null) {
    stereo = `<span class="c-muted">${f.channels <= 1 ? "mono" : "—"}</span>`;
  } else if (f.fake_stereo) {
    stereo = `<span class="c-bad has-tip" title="Both channels are identical: this &quot;stereo&quot; file is really mono duplicated onto two channels (fake stereo).">dual-mono</span>`;
  } else {
    stereo = `<span class="c-ok">${f.channels > 2 ? "multi" : "stereo"}</span>`;
  }

  // Clipping: severity-graded — yellow (a little), orange (a lot), red (heavy).
  // Sample-domain only; the true peak (inter-sample peak) has its own column,
  // since it's a level measurement that applies to every track, not just the
  // clipped ones.
  let clip: string;
  if (!f.clipping.clipped) {
    clip = `<span class="c-muted">none</span>`;
  } else {
    const n = f.clipping.clip_events;
    const cls = n >= 1000 ? "c-bad" : n >= 50 ? "c-mid" : "c-warn";
    const peak = Number.isFinite(f.clipping.peak_dbfs)
      ? f.clipping.peak_dbfs.toFixed(1)
      : "0.0";
    const title = `${n} clip event${n === 1 ? "" : "s"} (runs of ≥3 consecutive samples at full scale), peak ${peak} dBFS. Indicates a loud/clipped master — independent of whether the file is lossless.`;
    clip = `<span class="${cls} has-tip" title="${title}">${n} events</span>`;
  }

  const fmtCell = f.ext_mismatch
    ? `<td class="c-bad has-tip" title="Real container is ${f.format}, which does not match the file extension">${f.format}</td>`
    : `<td>${f.format}</td>`;

  return `<tr>
    <td class="reveal">${revealBtn(f.path)}</td>
    ${fname}
    ${fmtCell}
    ${badgeCell}
    <td>${(f.sample_rate / 1000).toFixed(1)}k</td>
    <td>${bits}</td>
    ${realCell}
    <td>${fmtDuration(f.duration_secs)}</td>
    ${detectionsTd(f.detections)}
    <td>${fmtCutoff(f)}</td>
    <td>${f.channels}</td>
    <td>${stereo}</td>
    <td>${clip}</td>
    ${truePeakCell(f.clipping.true_peak_dbtp)}
    ${drCell(f.dr_db)}
    ${showMd5 ? md5Cell(f.flac_md5) : ""}
    <td class="rowdel">${deleteBtn(f.path)}</td>
  </tr>`;
}

// --- Actions ----------------------------------------------------------------

// Merge freshly-analyzed files into the current result (adding, not replacing).
function mergeReport(report: FolderReport, targets: string[]): number {
  if (lastReport) {
    const existing = new Set(lastReport.files.map((f) => f.path));
    let added = 0;
    for (const f of report.files) {
      if (!existing.has(f.path)) {
        lastReport.files.push(f);
        added++;
      }
    }
    lastReport.files.sort((a, b) => a.path.localeCompare(b.path));
    lastReport.has_flac = lastReport.files.some((f) => f.flac_md5 != null);
    for (const t of targets) {
      if (!currentTargets.includes(t)) currentTargets.push(t);
    }
    renderReport(lastReport);
    return added;
  }
  currentTargets = targets.slice();
  renderReport(report);
  return report.files.length;
}

async function analyze(targets: string[]) {
  if (busy || targets.length === 0) return;
  setBusy(true, "Analyzing…");
  let wasCancel = false;
  let hadError = false;
  try {
    const report = await api.analyzePaths(targets);
    const added = mergeReport(report, targets);
    const total = lastReport ? lastReport.files.length : added;
    showToast(`Added ${added} ${added === 1 ? "file" : "files"} — ${total} in the list.`);
  } catch (e) {
    if (userCancelled || String(e).includes("cancelled")) {
      wasCancel = true;
    } else {
      hadError = true;
      showToast(String(e), "error");
    }
  } finally {
    setBusy(false);
    // Nothing was rendered on cancel or failure (e.g. a dropped file that
    // isn't a supported audio format) — restore whatever was on screen
    // before this attempt, otherwise the window is left blank.
    if (wasCancel || hadError) {
      if (lastReport && lastReport.files.length > 0) {
        renderReport(lastReport);
        if (wasCancel) showToast("Analysis cancelled — kept the existing list.");
      } else {
        showDropScreen();
        updateControls();
        if (wasCancel) showToast("Analysis cancelled.");
      }
    }
  }
}

async function generateSpectrograms() {
  if (busy || currentTargets.length === 0) return;
  setBusy(true, "Rendering spectrograms…", true); // keep results visible
  try {
    const s = await api.generateSpectrograms(currentTargets);
    if (userCancelled) {
      showToast(`Spectrograms cancelled — ${s.rendered}/${s.total} rendered.`);
    } else {
      const msg =
        `Rendered ${s.rendered}/${s.total} spectrograms` +
        (s.failed ? ` (${s.failed} failed)` : "") +
        ".";
      showToast(msg, s.failed ? "error" : "info");
    }
  } catch (e) {
    if (userCancelled || String(e).includes("cancelled")) {
      showToast("Spectrogram generation cancelled.");
    } else {
      showToast(String(e), "error");
    }
  } finally {
    setBusy(false);
  }
}

async function saveReport() {
  if (busy || !lastReport) return;
  const dest = await save({
    defaultPath: defaultSavePath(),
    // Only the file's stem is used (the backend always writes a matched
    // ".csv" + ".json" pair from it), so either extension works as a default.
    filters: [{ name: "Report (CSV + JSON)", extensions: ["csv", "json"] }],
  });
  if (typeof dest !== "string") return;
  try {
    await api.saveReport(dest, lastReport);
    showToast("Saved (CSV + JSON).");
  } catch (e) {
    showToast(String(e), "error");
  }
}

// Re-import a previously-saved JSON report, dropped onto the window, without
// re-analyzing any audio.
async function loadReport(path: string) {
  if (busy) return;
  setBusy(true, "Loading report…");
  try {
    const report = await api.loadReport(path);
    // Populate targets from the report's own file paths so "Generate
    // spectrograms" and re-analysis (e.g. dropping more files afterwards)
    // keep working, same as after a normal folder drop.
    currentTargets = report.files.map((f) => f.path);
    renderReport(report);
    showToast(`Loaded ${report.files.length} ${report.files.length === 1 ? "file" : "files"} from report.`);
  } catch (e) {
    showToast(String(e), "error");
  } finally {
    setBusy(false);
    if (!lastReport || lastReport.files.length === 0) {
      showDropScreen();
      updateControls();
    }
  }
}

async function pickFolder() {
  const dir = await open({ directory: true, multiple: false });
  if (typeof dir === "string") await analyze([dir]);
}

// --- Theme (Auto / Light / Dark), defaulting to the OS preference -----------

function applyTheme(t: Theme) {
  const root = document.documentElement;
  if (t === "auto") root.removeAttribute("data-theme");
  else root.setAttribute("data-theme", t);
  themeBtn.textContent =
    t === "auto" ? "◐ Auto" : t === "light" ? "☀ Light" : "☾ Dark";
  try {
    localStorage.setItem("theme", t);
  } catch {
    /* ignore */
  }
}

let theme: Theme = "auto";
try {
  const saved = localStorage.getItem("theme");
  if (saved === "light" || saved === "dark" || saved === "auto") theme = saved;
} catch {
  /* ignore */
}
applyTheme(theme);

// The window starts hidden (tauri.conf.json: visible=false) and is revealed
// here: styles are imported synchronously at the top of this module and the
// theme was just applied, so the first paint is fully styled.
// IMPORTANT: do NOT wrap this in requestAnimationFrame — WebKit suspends
// rendering callbacks for hidden windows, so the callback would never fire and
// the window would stay invisible forever.
const revealWindow = () => {
  const w = getCurrentWindow();
  w.show()
    .then(() => w.setFocus())
    .catch(() => {});
};
revealWindow();
// Defensive retry: a dev-server hot reload can race the first call.
window.setTimeout(revealWindow, 250);

// --- ffmpeg availability ----------------------------------------------------

(async () => {
  try {
    ffmpegAvailable = await api.ffmpegAvailable();
  } catch {
    ffmpegAvailable = false;
  }
  updateControls();
})();

// --- Wiring -----------------------------------------------------------------

pickBtn.addEventListener("click", pickFolder);
spectroBtn.addEventListener("click", generateSpectrograms);
saveBtn.addEventListener("click", saveReport);
resetBtn.addEventListener("click", reset);
cancelBtn.addEventListener("click", cancelTask);
themeBtn.addEventListener("click", () => {
  theme = theme === "auto" ? "light" : theme === "light" ? "dark" : "auto";
  applyTheme(theme);
});

// Delegated row-button clicks: magnifier (reveal) and trash (delete row).
table.addEventListener("click", (ev) => {
  const target = ev.target as HTMLElement;
  const reveal = target.closest<HTMLElement>(".reveal-btn");
  if (reveal) {
    const path = reveal.getAttribute("data-path");
    if (path) api.revealInFolder(path).catch((e) => showToast(String(e), "error"));
    return;
  }
  const del = target.closest<HTMLElement>(".delete-btn");
  if (del && lastReport) {
    const path = del.getAttribute("data-path");
    lastReport.files = lastReport.files.filter((f) => f.path !== path);
    if (lastReport.files.length === 0) {
      showDropScreen();
      updateControls();
    } else {
      renderReport(lastReport);
    }
  }
});

listen<Progress>("analyze://progress", (e) => updateProgress(e.payload, "Analyzing"));
listen<Progress>("spectro://progress", (e) => updateProgress(e.payload, "Rendering"));

getCurrentWebview().onDragDropEvent((event) => {
  const p = event.payload;
  if (busy) {
    // Dropping is disabled during analysis; show the busy overlay instead.
    dropzone.classList.remove("drag-over");
    dropGuard.classList.toggle("hidden", !(p.type === "enter" || p.type === "over"));
    return;
  }
  dropGuard.classList.add("hidden");
  if (p.type === "enter" || p.type === "over") {
    dropzone.classList.add("drag-over");
  } else if (p.type === "drop") {
    dropzone.classList.remove("drag-over");
    // A single previously-saved .json report reloads the table instead of
    // being analyzed as audio (there's no dedicated button for this — just
    // drop the file, same gesture as dropping a folder).
    if (p.paths.length === 1 && p.paths[0].toLowerCase().endsWith(".json")) {
      loadReport(p.paths[0]);
    } else if (p.paths.length > 0) {
      analyze(p.paths);
    }
  } else {
    dropzone.classList.remove("drag-over");
  }
});
