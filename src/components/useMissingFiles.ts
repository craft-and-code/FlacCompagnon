// Which of the listed tracks are no longer on disk.
//
// Deliberately *not* part of `FileAnalysis`. This is not a property of the
// audio — it is a property of the moment you are looking, and it goes stale
// the instant someone moves a folder in the Finder. Keeping it in frontend
// state rather than in the analysis record is also what guarantees it never
// reaches the CSV or the JSON report: those serialize the backend's record,
// and a "missing" flag saved into a report would be a lie the next time that
// report is opened.
//
// The check is a `metadata` call per path (`missing_paths` in
// commands/files.rs) — no decoding, so it stays cheap enough to run on a
// whole library at once.

import { useCallback, useEffect, useMemo, useState } from "react";

import * as api from "../api";
import { FilePresence, filePresenceKey, type FilePresenceSnapshot } from "./filePresence";
import { useLatest } from "./useLatest";

export interface UseMissingFilesArgs {
  /// Every path currently in the table.
  paths: string[];
  /// Called with the paths that were missing and are now back. Their tags and
  /// cover art were read (and failed) while they were gone, and the cache
  /// remembers a failed read as "nothing there" — so without this, a file
  /// that reappears comes back nameless and coverless until the whole listing
  /// is rebuilt.
  onReappeared: (paths: string[]) => void;
  /// True when the current listing came from a reloaded `.json` report rather
  /// than from a fresh analysis. A fresh analysis just read every one of
  /// these files, so they exist by construction and there is nothing to
  /// check; a reloaded report's paths were recorded who knows when.
  fromReport: boolean;
  onToast: (msg: string, kind?: "info" | "error") => void;
}

export function useMissingFiles({
  paths,
  fromReport,
  onReappeared,
  onToast,
}: UseMissingFilesArgs) {
  const [{ missing, checking }, setSnapshot] = useState<FilePresenceSnapshot>(() => ({
    missing: new Set(),
    checking: false,
  }));
  // Inline callbacks must not rebind the request controller or retrigger the
  // automatic scan on every render.
  const latest = useLatest({ paths, onReappeared, onToast });
  const [presence] = useState(() =>
    new FilePresence(
      api.missingPaths,
      setSnapshot,
      (back) => latest.current.onReappeared(back),
      (error) => latest.current.onToast(String(error), "error"),
    ),
  );

  /// The refresh button: same check, plus a spoken result. Silence would be
  /// indistinguishable from a button that does nothing, which is exactly the
  /// complaint this feature exists to answer.
  const refresh = useCallback(async () => {
    const current = latest.current.paths;
    await presence.check(current, (now) =>
      latest.current.onToast(
        now.size === 0
          ? `All ${current.length} file${current.length === 1 ? "" : "s"} are where they should be.`
          : `${now.size} of ${current.length} file${current.length === 1 ? "" : "s"} could not be found.`,
        now.size === 0 ? "info" : "error",
      ),
    );
  }, [presence, latest]);

  // A reloaded report is the case that motivates all of this: its paths were
  // written at some point in the past and nothing guarantees they still
  // resolve. Manual reorder must not cause another filesystem scan.
  const signature = useMemo(() => filePresenceKey(paths), [paths]);
  useEffect(() => {
    presence.reset();
    if (fromReport) void presence.check(latest.current.paths);
    return () => presence.reset();
  }, [signature, fromReport, presence, latest]);

  return { missing, checking, refresh };
}
