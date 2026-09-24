import type { FileAnalysis } from "../types";

export function HighFrequencyStereoCell({ f }: { f: FileAnalysis }) {
  const measurement = f.high_frequency_stereo;
  if (!measurement || !Number.isFinite(measurement.side_to_mid_db)) {
    return (
      <td
        className="c-muted has-tip"
        title="No high-frequency stereo-width reading: the layout or sample rate is unsupported, the signal is too quiet or too short, samples are invalid, or this is an older saved report."
      >
        —
      </td>
    );
  }
  const sideToMid = measurement.side_to_mid_db.toFixed(1);
  const reference = Number.isFinite(measurement.reference_side_to_mid_db)
    ? measurement.reference_side_to_mid_db.toFixed(1)
    : "unavailable";
  const narrowedBlocks = (measurement.narrowed_block_fraction * 100).toFixed(0);
  const floor =
    measurement.side_to_mid_db <= -120
      ? " -120 dB is the reporting floor, including zero Side."
      : "";
  const title = `Experimental high-frequency stereo-width cue, aggregated over eligible complete blocks: Side/Mid ${sideToMid} dB in the 6–20 kHz band (limited by Nyquist); reference Side/Mid ${reference} dB from 1.5 to 5 kHz; ${narrowedBlocks}% of eligible 500 ms blocks are narrowed. ${measurement.narrowed ? "A persistent high-frequency narrowing cue can result from intensity stereo, but identifies neither a codec nor a defect." : "The eligible blocks do not meet the persistent narrowing rule."}${floor}`;
  return (
    <td className={`${measurement.narrowed ? "c-warn " : ""}has-tip`} title={title}>
      {sideToMid} dB
    </td>
  );
}
