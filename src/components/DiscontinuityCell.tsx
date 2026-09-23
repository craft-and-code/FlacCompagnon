import type { EventSummary } from "../types";
import { discontinuityLabel } from "../format";

export function DiscontinuityCell({ summary, kind }: {
  summary: EventSummary | null | undefined;
  kind: "clicks" | "dropouts";
}) {
  if (!summary) {
    return <td className="c-muted has-tip" title="No reading: older report, DSD, insufficient audio context, invalid samples or unsupported stream.">—</td>;
  }
  const criterion = kind === "clicks"
    ? "Isolated pulse candidates up to 0.5 ms, with abrupt edges well above the surrounding signal changes."
    : "Abrupt digital-silence candidates lasting 2–250 ms, with signal before and after.";
  const locations = summary.events.map((event) =>
    `Ch ${event.channel} · ${event.start_secs.toFixed(3)} s · ${(event.duration_secs * 1000).toFixed(3)} ms`);
  const limited = summary.count > summary.events.length ? `\nFirst ${summary.events.length} locations shown.` : "";
  const title = `${criterion} Suspected events require listening: percussion or intentional edits can resemble defects. Counts are per channel; no candidates does not guarantee defect-free audio. Events near file boundaries lack context.\n${locations.join("\n")}${limited}`;
  return <td className={`${summary.count > 0 ? "c-warn" : "c-muted"} has-tip`} title={title}>{discontinuityLabel(summary, kind)}</td>;
}
