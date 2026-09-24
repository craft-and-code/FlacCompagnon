import type { LoudnessPeak } from "../types";
import { loudnessPeakValue } from "../format";

export function LoudnessPeakCell({
  peak,
  mode,
}: {
  peak: LoudnessPeak | null | undefined;
  mode: "momentary" | "short-term";
}) {
  const value = loudnessPeakValue(peak);
  const seconds = mode === "momentary" ? 0.4 : 3;
  const window = mode === "momentary" ? "400 ms" : "3 s";
  if (value == null || !peak) {
    return (
      <td
        className="c-muted has-tip"
        title={`No ${mode} loudness maximum: a complete ${window} window of non-silent mono/stereo audio is required. Unsupported rates, invalid or unrepresentable samples, skipped decoding and older reports also have no reading.`}
      >
        —
      </td>
    );
  }
  const title = `Maximum ${mode} loudness: ${value.toFixed(1)} LUFS over a ${window} rectangular window (EBU Tech 3341). K-weighted, ungated, measured at every decoded frame. Window: ${peak.start_secs.toFixed(3)}–${(peak.start_secs + seconds).toFixed(3)} s. Complete real-audio windows only; no end padding. This is the loudest local window, not the integrated programme loudness or an instantaneous sample peak.`;
  return (
    <td className="has-tip" title={title}>
      {Number(value.toFixed(1)).toFixed(1)}
    </td>
  );
}
