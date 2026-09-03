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

import { useCallback, useEffect, useRef, useState } from "react";

import * as api from "../api";

export interface UseMissingFilesArgs {
  /// Every path currently in the table.
  paths: string[];
  /// True when the current listing came from a reloaded `.json` report rather
  /// than from a fresh analysis. A fresh analysis just read every one of
  /// these files, so they exist by construction and there is nothing to
  /// check; a reloaded report's paths were recorded who knows when.
  fromReport: boolean;
  onToast: (msg: string, kind?: "info" | "error") => void;
}

export function useMissingFiles({ paths, fromReport, onToast }: UseMissingFilesArgs) {
  const [missing, setMissing] = useState<Set<string>>(() => new Set());
  const [checking, setChecking] = useState(false);

  // `paths` is a fresh array on every render, so effects and callbacks read it
  // through a ref instead of listing it as a dependency — otherwise the
  // automatic check below would re-run on every render forever.
  const pathsRef = useRef(paths);
  pathsRef.current = paths;

  const check = useCallback(async () => {
    const current = pathsRef.current;
    if (current.length === 0) {
      setMissing(new Set());
      return;
    }
    setChecking(true);
    try {
      setMissing(new Set(await api.missingPaths(current)));
    } catch (e) {
      onToast(String(e), "error");
    } finally {
      setChecking(false);
    }
  }, [onToast]);

  /// The refresh button: same check, plus a spoken result. Silence would be
  /// indistinguishable from a button that does nothing, which is exactly the
  /// complaint this feature exists to answer.
  const refresh = useCallback(async () => {
    const before = pathsRef.current.length;
    await check();
    if (before === 0) return;
    // Read back through the state setter rather than `missing`, which is the
    // value captured when this callback was created, not the one just set.
    setMissing((now) => {
      onToast(
        now.size === 0
          ? `All ${before} file${before === 1 ? "" : "s"} are where they should be.`
          : `${now.size} of ${before} file${before === 1 ? "" : "s"} could not be found.`,
        now.size === 0 ? "info" : "error",
      );
      return now;
    });
  }, [check, onToast]);

  // A reloaded report is the case that motivates all of this: its paths were
  // written at some point in the past and nothing guarantees they still
  // resolve. Keyed on the listing's identity (its length and first path is
  // enough to tell one load from another) rather than on `paths` itself,
  // which changes identity on every render.
  const signature = `${fromReport}|${paths.length}|${paths[0] ?? ""}`;
  useEffect(() => {
    if (!fromReport) {
      setMissing(new Set());
      return;
    }
    void check();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [signature, check]);

  return { missing, checking, refresh };
}
