import type { DraftRevision } from "./draftRevision";

// Native credential writes may wait for an unlock prompt. Clear only the
// submitted draft so text entered during that wait remains available.
export async function finishCredentialDraft(
  revision: DraftRevision,
  change: () => Promise<boolean>,
  clear: () => void,
): Promise<void> {
  const submitted = revision.capture();
  if ((await change()) && revision.isCurrent(submitted)) {
    revision.touch();
    clear();
  }
}
