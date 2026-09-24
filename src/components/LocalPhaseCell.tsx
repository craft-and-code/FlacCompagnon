import type { FileAnalysis, PhaseSummary } from "../types";
import { localPhaseMinimum, phaseCorrelationLabel } from "../format";

function evidence(label: string, summary: PhaseSummary | null): string {
  if (!summary) return `${label}: unavailable (no eligible window).`;
  return `${label}: aggregate ${phaseCorrelationLabel(summary.correlation)}; minimum ${phaseCorrelationLabel(summary.minimum_correlation)} at ${summary.minimum_start_secs.toFixed(3)} s; ${(summary.opposed_fraction * 100).toFixed(1)}% of ${summary.eligible_windows} eligible windows at or below -0.5.`;
}

export function LocalPhaseCell({ f, mode }: { f: FileAnalysis; mode: "local" | "bands" }) {
  const phase = f.local_phase;
  const minimum = localPhaseMinimum(phase, mode);
  if (!phase || minimum == null) {
    return (
      <td
        className="c-muted has-tip"
        title="No local phase reading: stereo audio with a complete window and sufficient signal in both channels is required. The layout or rate may be unsupported, samples invalid, or this may be an older report."
      >
        —
      </td>
    );
  }
  const lines =
    mode === "local"
      ? [evidence(`20–${Math.min(20000, f.sample_rate / 2)} Hz`, phase.broadband)]
      : phase.bands.map((band) =>
          band.high_hz <= band.low_hz
            ? `Above Nyquist (${band.low_hz} Hz band): unavailable.`
            : evidence(`${band.low_hz}–${band.high_hz} Hz`, band.summary),
        );
  const title = [
    mode === "local"
      ? "Lowest local broadband L/R correlation."
      : "Lowest local L/R correlation across the four bands.",
    "+1: aligned; 0: uncorrelated or quadrature; -1: opposed. Correlation is not a phase angle or a defect verdict.",
    `${(phase.window_secs * 1000).toFixed(1)} ms Hann windows, 50% overlap; ${phase.analyzed_windows} complete windows examined. Both channels must exceed -60 dBFS RMS in the measured range. DC is removed. Partial tails are excluded.`,
    "Locations are window starts, not exact event boundaries. Percentages describe eligible overlapping windows, not track duration.",
    ...lines,
  ].join("\n");
  return (
    <td className="has-tip" title={title}>
      {phaseCorrelationLabel(minimum)}
    </td>
  );
}
