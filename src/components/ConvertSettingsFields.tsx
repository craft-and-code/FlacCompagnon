// The form describing what a conversion batch should produce: target format,
// its one format-specific knob, and the two options that affect the output
// tree rather than the audio.
//
// Split out of ConvertPanel once the encoding options landed: the panel's own
// job is the imported list, the drop zone and running the batch, and the
// settings had grown into a second job with its own vocabulary (four format
// notes, two label tables). Purely presentational — every value and setter is
// a prop, all of them owned by useConvertPanel.

import type { ConvertFormat, FlacEffort } from "../types";
import "./ConvertSettingsFields.css";

const OPUS_BITRATES = [96, 128, 160, 192, 256];
const MP3_BITRATES = [128, 192, 256, 320];

const FORMAT_LABELS: Record<ConvertFormat, string> = {
  flac: "FLAC (lossless)",
  opus: "Opus (lossy, recommended)",
  mp3: "MP3 (lossy, compatible)",
  wav: "WAV (16-bit PCM)",
};

/// One note per format, shown under the picker — always, not on hover.
///
/// Choosing the format is the only decision in this panel with lasting
/// consequences: it decides whether the copy can ever be turned back into the
/// original. That is exactly the sort of thing a tooltip hides from anyone who
/// doesn't already know to look for it, so it stays on screen.
///
/// Each note answers the same two questions in the same order — *what does
/// this cost me* and *when would I pick it* — so the four can be compared by
/// reading the same position in each, rather than four differently-shaped
/// paragraphs. The FLAC and WAV ones also carry what used to be their only
/// note ("no bitrate to choose"), since the bitrate select disappears for
/// them and its absence otherwise looks like a missing control.
const FORMAT_NOTES: Record<ConvertFormat, string> = {
  flac: "Keeps every bit of the original, at approximately half its size. No bitrate to choose — the size follows the music. Pick it to archive, or whenever the copy may itself become a source.",
  opus: "Discards what you are unlikely to hear, for roughly a fifth of the size. The best quality per megabyte of the four, but younger, so some older players and car stereos don't read it. Pick it for a phone or a music player that supports it.",
  mp3: "Also discards detail, and less efficiently than Opus at the same bitrate — its advantage is that everything made in the last twenty-five years can play it. Pick it when compatibility matters more than size.",
  wav: "No compression at all: the largest files of the four, fixed at 16-bit whatever the source's own depth. Pick it only for a tool that refuses everything else — for keeping the audio intact, FLAC does the same job far smaller.",
};

/// FLAC search effort, named for what it costs rather than numbered.
///
/// libFLAC's familiar `-0`..`-8` are not used here on purpose: this app encodes
/// with `flacenc`, whose knobs are its own, so borrowing those labels would
/// promise an equivalence the output doesn't have. Names also say the one thing
/// the numbers never did — that none of this touches the audio.
const EFFORT_LABELS: Record<FlacEffort, string> = {
  fast: "Fast (larger files)",
  balanced: "Balanced (recommended)",
  maximum: "Maximum (slowest, smallest)",
};

export interface ConvertSettingsFieldsProps {
  format: ConvertFormat;
  bitrateKbps: number | null;
  flacEffort: FlacEffort;
  preserveModtime: boolean;
  copyOthers: boolean;
  /// A batch is running: every control here is frozen for its duration.
  busy: boolean;
  onSetFormat: (f: ConvertFormat) => void;
  onSetBitrateKbps: (kbps: number | null) => void;
  onSetFlacEffort: (e: FlacEffort) => void;
  onSetPreserveModtime: (v: boolean) => void;
  onSetCopyOthers: (v: boolean) => void;
}

export function ConvertSettingsFields({
  format,
  bitrateKbps,
  flacEffort,
  preserveModtime,
  copyOthers,
  busy,
  onSetFormat,
  onSetBitrateKbps,
  onSetFlacEffort,
  onSetPreserveModtime,
  onSetCopyOthers,
}: ConvertSettingsFieldsProps) {
  const bitratePresets = format === "opus" ? OPUS_BITRATES : format === "mp3" ? MP3_BITRATES : null;

  return (
    <div className="convert-settings">
      <label className="convert-field">
        <span>Format</span>
        <select
          value={format}
          disabled={busy}
          onChange={(ev) => onSetFormat(ev.target.value as ConvertFormat)}
        >
          {(Object.keys(FORMAT_LABELS) as ConvertFormat[]).map((f) => (
            <option value={f} key={f}>
              {FORMAT_LABELS[f]}
            </option>
          ))}
        </select>
      </label>

      {/* Directly under the picker it describes, and above the per-format
          control below — a note about FLAC that sat under FLAC's own effort
          select would read as a caption for that select instead.
          Unconditional: every format has a note, so a guard here would always
          pass and only suggest otherwise. */}
      <p className="convert-field-note">{FORMAT_NOTES[format]}</p>

      {/* One slot, whichever knob the chosen format actually has: a bitrate
          for the lossy pair, a search effort for FLAC. They are mutually
          exclusive by construction (WAV has neither), so they share the
          position rather than each reserving one and leaving a gap for three
          formats out of four. */}
      {bitratePresets && (
        <label className="convert-field">
          <span>Bitrate</span>
          <select
            value={bitrateKbps ?? "auto"}
            disabled={busy}
            onChange={(ev) =>
              onSetBitrateKbps(ev.target.value === "auto" ? null : Number(ev.target.value))
            }
          >
            <option value="auto">Auto (recommended)</option>
            {bitratePresets.map((kbps) => (
              <option value={kbps} key={kbps}>
                {kbps} kbps
              </option>
            ))}
          </select>
        </label>
      )}

      {format === "flac" && (
        <label
          className="convert-field"
          title="Only changes how long encoding takes and how small the result is — every level decodes back to the exact original"
        >
          <span>Compression</span>
          <select
            value={flacEffort}
            disabled={busy}
            onChange={(ev) => onSetFlacEffort(ev.target.value as FlacEffort)}
          >
            {(Object.keys(EFFORT_LABELS) as FlacEffort[]).map((e) => (
              <option value={e} key={e}>
                {EFFORT_LABELS[e]}
              </option>
            ))}
          </select>
        </label>
      )}

      <label
        className="convert-checkbox"
        title="Covers, playlists, spectrograms, and any other non-audio file"
      >
        <input
          type="checkbox"
          checked={copyOthers}
          disabled={busy}
          onChange={(ev) => onSetCopyOthers(ev.target.checked)}
        />
        <span>Also copy other files</span>
      </label>

      {/* Not filed under FLAC despite being libFLAC's `--preserve-modtime` by
          name: it describes the file that gets written, not the codec that
          writes it, and applies to all four formats. Its neighbour here is the
          other option that says what happens to the output tree rather than to
          the audio. */}
      <label
        className="convert-checkbox"
        title="Give each converted file the same last-modified date as its source, so the copies sort like the originals"
      >
        <input
          type="checkbox"
          checked={preserveModtime}
          disabled={busy}
          onChange={(ev) => onSetPreserveModtime(ev.target.checked)}
        />
        <span>Keep original file dates</span>
      </label>
    </div>
  );
}
