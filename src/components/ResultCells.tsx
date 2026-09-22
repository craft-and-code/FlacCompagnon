// The result table's non-trivial cells: the ones that carry a colour, a
// tooltip explaining the reasoning, or both. Grouped here so a row stays
// readable and each cell's colour rules live next to the text that explains
// them.

import type { ClippingInfo, Detections, FileAnalysis, FlacMd5Status } from "./../types";
import { detectionLabels } from "../format";
import "./ResultCells.css";

export function DetectionsCell({ d }: { d: Detections }) {
  const tags = detectionLabels(d).map((label) => ({
    cls: label === "Unknown" ? "c-muted" : `t-${label.toLowerCase()}`,
    label,
  }));

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
/// track whether or not sample-domain clipping fired.
///
/// Four tiers, each pinned to a reference rather than to a round number:
///
/// | dBTP | Colour | Meaning |
/// |---|---|---|
/// | ≤ −1.0 | green | meets the EBU R128 ceiling every major platform adopted |
/// | −1.0 … 0.0 | neutral | sound, just without the recommended headroom |
/// | 0.0 … +1.0 | amber | real inter-sample overs, but small |
/// | > +1.0 | red | enough to distort when re-encoded to a lossy format |
///
/// Red used to start at 0 dBTP, which was technically defensible and
/// practically useless: essentially every commercial master since the 1990s
/// clears 0 dBTP, so the colour fired on almost everything and stopped
/// carrying information. The line the industry actually draws is −1 dBTP
/// (EBU R128, and Spotify/Apple Music/Tidal after it), and the reason it sits
/// there is specific: a lossy encoder *adds* overshoot of its own, so a file
/// at +1 dBTP can leave an AAC encoder at +2 or +3. Below that, an over is a
/// fact worth showing and not a problem worth alarming about.
export function TruePeakCell({ dbtp }: { dbtp: number }) {
  if (!Number.isFinite(dbtp)) return <td className="c-muted">—</td>;
  // Classify off the *rounded* value, not the raw one: a raw +1.02 dBTP
  // still displays as "+1.0" at one decimal, so coloring it red anyway would
  // show two identical-looking "+1.0 dBTP" cells with no visible reason one
  // is flagged and the other isn't. What's on screen has to be what decides
  // the color.
  const rounded = Number(dbtp.toFixed(1));
  const over = rounded > 0;
  const cls = rounded > 1 ? "c-bad" : over ? "c-mid" : rounded <= -1 ? "c-ok" : "";
  const text = `${over ? "+" : ""}${rounded.toFixed(1)} dBTP`;
  const title =
    rounded > 1
      ? `True peak ${text}: the waveform a DAC reconstructs between samples overshoots full scale by more than 1 dB. Lossy encoders add overshoot of their own, so converting this file can push it well past the point of audible distortion. Independent of whether the file is lossless.`
      : over
        ? `Inter-sample over: no stored sample reaches full scale, but the true peak (4x-oversampled, BS.1770-style) reaches ${text}. Common on modern masters and benign at this size — it only becomes a problem past +1 dBTP, where re-encoding to a lossy format can distort.`
        : rounded <= -1
          ? `True peak ${text}: at or below the −1 dBTP ceiling of EBU R128, which Spotify, Apple Music and Tidal all adopted. Enough headroom to survive lossy encoding intact.`
          : `True peak ${text}: below full scale, so no inter-sample clipping — but without the −1 dBTP of headroom broadcast and streaming delivery ask for.`;
  return (
    <td className={`${cls} has-tip`} title={title}>
      {text}
    </td>
  );
}

/// A whole-file fingerprint, shown monospaced so digits line up between rows
/// — the only way a column of hex is scannable at all.
///
/// The full value is always rendered: these columns are hidden by default, so
/// a user who turned one on wants the hash, not an abbreviation they then
/// have to hover to complete. The `title` repeats it anyway, for the case
/// where the column has been narrowed.
export function FileHashCell({ hash }: { hash: string | null }) {
  if (!hash) return <td className="c-muted">—</td>;
  return (
    <td className="file-hash" title={hash}>
      {hash}
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
      // Red, not amber. `Error` means the file could not be decoded at all —
      // the decoder gave up part-way, which is how a truncated or corrupted
      // stream shows up. That is not a lesser finding than `Mismatch`: a
      // mismatch says the audio changed, this says the audio cannot even be
      // read to the end. Amber read as "worth a look" for a file that is
      // damaged.
      return (
        <td className="c-bad has-tip" title={m.detail}>
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

/// Occupied integer bits describe the stored samples, not the source master.
/// Padding is flagged; a full-width result is neutral because noise counts too.
export function RealBitsCell({ f }: { f: FileAnalysis }) {
  if (f.real_bit_depth == null) {
    const reason = f.error == null && f.cutoff_hz != null && f.clipping.peak === 0
      ? "Digital silence: all decoded samples are zero. Effective bit depth cannot be determined; this is not a 1-bit recording."
      : "Integer bit depth could not be measured. No original resolution is verified.";
    return <td className="c-muted has-tip" title={reason}>—</td>;
  }
  if (f.declared_bits != null && f.real_bit_depth < f.declared_bits) {
    return (
      <td
        className="c-bad has-tip"
        title={`The decoded samples follow a ${f.real_bit_depth}-bit grid within the declared ${f.declared_bits}-bit container. This can be exact zero-padding or a lower-bit-depth source masked by low-level dither; hover the Detection cell for the measured case.`}
      >
        {f.real_bit_depth}-bit
      </td>
    );
  }
  return (
    <td className="has-tip" title="Occupied bits in the decoded integer samples. Dither, noise or processing can occupy these bits without increasing the original recording resolution.">
      {f.real_bit_depth}-bit
    </td>
  );
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
