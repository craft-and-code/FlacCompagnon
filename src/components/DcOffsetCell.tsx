import type { FileAnalysis } from "../types";
import { dcOffsetMagnitude, dcOffsetPercent } from "../format";

export function DcOffsetCell({ f }: { f: FileAnalysis }) {
  const magnitude = dcOffsetMagnitude(f.dc_offset);
  if (magnitude == null || !f.dc_offset) {
    return (
      <td
        className="c-muted has-tip"
        title="No DC offset reading: empty or invalid audio, unsupported channel count, failed or skipped decoding, or an older saved report."
      >
        —
      </td>
    );
  }
  const channels = f.dc_offset.channel_means.map(
    (mean, index) => `Ch ${index + 1}: ${dcOffsetPercent(mean, true)}%`,
  );
  const title =
    "DC offset: largest absolute channel mean as a percentage of full scale. " +
    "Signed channel means include every decoded sample, including silence. " +
    "Short excerpts and incomplete low-frequency cycles can have a nonzero mean; " +
    "opposite biases at different times can cancel. 0.000 reflects display precision. " +
    "This is a measurement, not an authenticity or audibility verdict.\n" +
    channels.join("\n");
  return (
    <td className="has-tip" title={title}>
      {dcOffsetPercent(magnitude)}
    </td>
  );
}
