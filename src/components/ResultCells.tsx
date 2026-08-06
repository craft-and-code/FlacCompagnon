// The result table's non-trivial cells: the ones that carry a colour, a
// tooltip explaining the reasoning, or both. Grouped here so a row stays
// readable and each cell's colour rules live next to the text that explains
// them.

import type { ClippingInfo, Detections, FileAnalysis, FlacMd5Status } from "./../types";

export function DetectionsCell({ d }: { d: Detections }) {
  // Ordered least to most severe — the container's bit depth, then its sample
  // rate, then the audio itself — matching the colour ramp amber/orange/red.
  const tags: { cls: string; label: string }[] = [];
  if (d.upscaling) tags.push({ cls: "t-upscaled", label: "Upscaled" });
  if (d.upsampling) tags.push({ cls: "t-upsampled", label: "Upsampled" });
  if (d.transcoding === "detected") tags.push({ cls: "t-transcoded", label: "Transcoded" });
  else if (d.transcoding === "suspected") tags.push({ cls: "t-suspected", label: "Transcoded?" });
  if (tags.length === 0) tags.push({ cls: "t-clean", label: "Clean" });

  return (
    <td className="detections" title={d.detail}>
      {tags.map((t, i) => (
        <span className={`tag ${t.cls}`} key={t.cls}>
          {i > 0 && " "}
          {t.label}
        </span>
      ))}
    </td>
  );
}

const DR_TIP =
  "Dynamic range (crest of the loud passages): peak level vs the RMS of the loudest 20% of ~3 s blocks. " +
  "High values mean a dynamic master (Full Dynamic Range editions); low values a compressed “loudness war” master. " +
  "Independent of whether the file is lossless.";

/// >= 12 dB green (dynamic master), 8-12 neutral, < 8 amber (loudness-war).
export function DynamicRangeCell({ dr }: { dr: number | null }) {
  if (dr == null || !Number.isFinite(dr)) return <td className="c-muted">—</td>;
  const cls = dr >= 12 ? "c-ok" : dr >= 8 ? "" : "c-warn";
  return (
    <td className={`${cls} has-tip`} title={DR_TIP}>
      {dr.toFixed(1)} dB
    </td>
  );
}

/// True peak is a level measurement, not an event count, so it shows for every
/// track whether or not sample-domain clipping fired. Above 0 dBTP the
/// reconstructed waveform overshoots full scale *between* stored samples — a
/// measured fact rather than a heuristic, hence the same red as other
/// confirmed issues.
export function TruePeakCell({ dbtp }: { dbtp: number }) {
  if (!Number.isFinite(dbtp)) return <td className="c-muted">—</td>;
  // Classify off the *rounded* value, not the raw one: a raw +0.02 dBTP
  // still displays as "0.0" at one decimal, so coloring it red anyway would
  // show two identical-looking "0.0 dBTP" cells with no visible reason one
  // is flagged and the other isn't. What's on screen has to be what decides
  // the color.
  const rounded = Number(dbtp.toFixed(1));
  const over = rounded > 0;
  const cls = over ? "c-bad" : rounded <= -1 ? "c-ok" : "";
  const text = `${over ? "+" : ""}${rounded.toFixed(1)} dBTP`;
  const title = over
    ? `Inter-sample over: no stored sample reaches full scale, but the true peak (4x-oversampled, BS.1770-style) reaches ${text} — the waveform a DAC reconstructs between samples overshoots 0 dBFS. A sign of an over-loud master, independent of whether the file is lossless.`
    : `True peak (4x-oversampled inter-sample peak): ${text}. At or below 0 dBTP means the reconstructed waveform stays under full scale — no inter-sample clipping.`;
  return (
    <td className={`${cls} has-tip`} title={title}>
      {text}
    </td>
  );
}

export function Md5Cell({ m }: { m: FlacMd5Status | null }) {
  if (!m) return <td className="c-muted">—</td>;
  switch (m.state) {
    case "Match":
      return <td className="c-ok">✓ OK</td>;
    case "Mismatch":
      return <td className="c-bad">✗ Mismatch</td>;
    case "NoSignature":
      return <td className="c-muted">No signature</td>;
    case "Present":
      return <td className="c-warn">Present</td>;
    case "Error":
      return (
        <td className="c-warn has-tip" title={m.detail}>
          Error
        </td>
      );
  }
}

/// Sample-domain clipping, severity-graded: amber (a little), orange (a lot),
/// red (heavy).
export function ClippingCell({ c }: { c: ClippingInfo }) {
  if (!c.clipped) {
    return (
      <td>
        <span className="c-muted">none</span>
      </td>
    );
  }
  const n = c.clip_events;
  const cls = n >= 1000 ? "c-bad" : n >= 50 ? "c-mid" : "c-warn";
  const peak = Number.isFinite(c.peak_dbfs) ? c.peak_dbfs.toFixed(1) : "0.0";
  const title = `${n} clip event${n === 1 ? "" : "s"} (runs of ≥3 consecutive samples at full scale), peak ${peak} dBFS. Indicates a loud/clipped master — independent of whether the file is lossless.`;
  return (
    <td>
      <span className={`${cls} has-tip`} title={title}>
        {n} events
      </span>
    </td>
  );
}

/// Green when the content really uses the depth it claims, red when the low
/// bits are always zero (i.e. it was upscaled).
export function RealBitsCell({ f }: { f: FileAnalysis }) {
  if (f.real_bit_depth == null) return <td className="c-muted">—</td>;
  if (f.declared_bits != null && f.real_bit_depth < f.declared_bits) {
    return (
      <td
        className="c-bad has-tip"
        title={`Only ${f.real_bit_depth} of the declared ${f.declared_bits} bits carry real information — the low bits are always zero (the content was upscaled to a higher bit depth).`}
      >
        {f.real_bit_depth}-bit
      </td>
    );
  }
  return <td className="c-ok">{f.real_bit_depth}-bit</td>;
}

export function StereoCell({ f }: { f: FileAnalysis }) {
  if (f.fake_stereo == null) {
    return (
      <td>
        <span className="c-muted">{f.channels <= 1 ? "mono" : "—"}</span>
      </td>
    );
  }
  if (f.fake_stereo) {
    return (
      <td>
        <span
          className="c-bad has-tip"
          title={'Both channels are identical: this "stereo" file is really mono duplicated onto two channels (fake stereo).'}
        >
          dual-mono
        </span>
      </td>
    );
  }
  return (
    <td>
      <span className="c-ok">{f.channels > 2 ? "multi" : "stereo"}</span>
    </td>
  );
}

/// Custom chip rather than the official DSD / Hi-Res Audio logos, which are
/// trademarked. Granted only when no detection contradicts the claim.
export function QualityBadgeCell({ badge }: { badge: string | null }) {
  if (!badge) return <td className="c-muted">—</td>;
  const unverified = badge.includes("unverified");
  const dsdSource = badge.includes("DSD source");
  const title = unverified
    ? "Container header is authentic, but the content could not be analyzed (ffmpeg not found)."
    : dsdSource
      ? "Hi-Res PCM carrying the sigma-delta noise signature of a DSD master — verified by analysis."
      : "Verified by analysis: the claimed quality is not contradicted by any detection.";
  const label = badge.replace(" (unverified)", "?").replace(" (DSD source)", "·DSD");
  return (
    <td>
      <span className={`qbadge${unverified ? " q-unk" : ""} has-tip`} title={title}>
        {label}
      </span>
    </td>
  );
}
