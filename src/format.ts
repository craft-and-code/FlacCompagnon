// Pure, DOM-free helpers for formatting values and building table-cell HTML.

import type { Detections, FileAnalysis, FlacMd5Status } from "./types";

/// Audio extensions, used to strip the extension from a single dropped file
/// when suggesting a CSV name.
export const AUDIO_EXTS = [
  "flac", "wav", "wave", "aif", "aiff", "aifc", "alac", "m4a", "mp4", "caf",
  "ogg", "oga", "mp3", "aac",
];

export function escapeHtml(s: string): string {
  return s.replace(
    /[&<>"']/g,
    (c) =>
      ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" })[
        c
      ]!,
  );
}

export function fmtDuration(secs: number): string {
  const t = Math.round(secs);
  const m = Math.floor(t / 60);
  const s = t % 60;
  return `${m}:${String(s).padStart(2, "0")}`;
}

export function fmtCutoff(f: FileAnalysis): string {
  if (f.cutoff_hz == null || f.cutoff_ratio == null) return "—";
  return `${(f.cutoff_hz / 1000).toFixed(1)} kHz (${Math.round(
    f.cutoff_ratio * 100,
  )}%)`;
}

// Small magnifier icon for the "reveal in file browser" row button.
const MAG_ICON = `<svg viewBox="0 0 16 16" width="14" height="14" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round"><circle cx="7" cy="7" r="4.3"/><line x1="10.4" y1="10.4" x2="14" y2="14"/></svg>`;

// Trash icon for removing a row before exporting.
const TRASH_ICON = `<svg viewBox="0 0 16 16" width="14" height="14" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"><path d="M2.5 4h11"/><path d="M6 4V2.6h4V4"/><path d="M4.2 4l.6 9.4a1 1 0 0 0 1 .9h4.4a1 1 0 0 0 1-.9L11.8 4"/><path d="M6.6 7v4.2M9.4 7v4.2"/></svg>`;

export function revealBtn(path: string): string {
  return `<button class="reveal-btn" data-path="${escapeHtml(path)}" title="Reveal in file browser">${MAG_ICON}</button>`;
}

export function deleteBtn(path: string): string {
  return `<button class="delete-btn" data-path="${escapeHtml(path)}" title="Remove this row">${TRASH_ICON}</button>`;
}

export function detectionsTd(d: Detections): string {
  const tags: string[] = [];
  if (d.upscaling) tags.push(`<span class="tag t-up">Upscaled</span>`);
  if (d.upsampling) tags.push(`<span class="tag t-ups">Upsampled</span>`);
  if (d.transcoding === "detected") {
    tags.push(`<span class="tag t-tr">Transcoded</span>`);
  } else if (d.transcoding === "suspected") {
    tags.push(`<span class="tag t-sus">Transcoded?</span>`);
  }
  if (tags.length === 0) tags.push(`<span class="tag t-ok">Clean</span>`);
  return `<td class="detections" title="${escapeHtml(d.detail)}">${tags.join(" ")}</td>`;
}

export function md5Cell(m: FlacMd5Status | null): string {
  if (!m) return `<td class="c-muted">—</td>`;
  switch (m.state) {
    case "Match":
      return `<td class="c-ok">✓ OK</td>`;
    case "Mismatch":
      return `<td class="c-bad">✗ Mismatch</td>`;
    case "NoSignature":
      return `<td class="c-muted">No signature</td>`;
    case "Present":
      return `<td class="c-warn">Present</td>`;
    case "Error":
      return `<td class="c-warn has-tip" title="${escapeHtml(m.detail)}">Error</td>`;
  }
}

/// The deepest folder that contains every one of `paths`. With a single folder
/// of files this is that folder; across several folders it is their common
/// ancestor. Recomputed as files are added.
export function commonDir(paths: string[]): string {
  if (paths.length === 0) return "";
  const sep = paths[0].includes("\\") ? "\\" : "/";
  const dirs = paths.map((p) => p.split(/[\\/]/).slice(0, -1)); // drop the filename
  let common = dirs[0];
  for (const d of dirs.slice(1)) {
    let i = 0;
    while (i < common.length && i < d.length && common[i] === d[i]) i++;
    common = common.slice(0, i);
  }
  return common.join(sep) || sep;
}

/// Suggest a CSV file name from a single dropped path (folder or file name,
/// with an audio extension stripped). `fallback` is used otherwise.
export function csvNameFrom(path: string, fallback = "FlacCompagnon"): string {
  const segments = path.split(/[\\/]/).filter(Boolean);
  let name = segments.length ? segments[segments.length - 1] : fallback;
  const m = name.match(/\.([A-Za-z0-9]+)$/);
  if (m && AUDIO_EXTS.includes(m[1].toLowerCase())) {
    name = name.slice(0, -(m[1].length + 1));
  }
  return `${name || fallback}.csv`;
}
