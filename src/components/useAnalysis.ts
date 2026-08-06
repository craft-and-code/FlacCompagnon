// The analysis session: what's been analyzed, in what order it's shown, and
// the busy/progress state of whatever backend task is running.
//
// Display order is kept separate from `report.files` (which the backend
// returns path-sorted) because dragging rows rearranges it, and that manual
// order is what CSV/JSON/M3U exports use — "the order you see is the order you
// get". Re-analyzing keeps it: existing rows stay put, new files are appended.

import { useCallback, useMemo, useRef, useState } from "react";

import type { FolderReport, Progress } from "../types";
import * as api from "../api";

export interface UseAnalysisArgs {
  onToast: (msg: string, kind?: "info" | "error") => void;
  /// Called with the paths that vanished, so caches and the selection can drop
  /// them.
  onFilesChanged: (present: Set<string>) => void;
}

export function useAnalysis({ onToast, onFilesChanged }: UseAnalysisArgs) {
  const [report, setReport] = useState<FolderReport | null>(null);
  const [targets, setTargets] = useState<string[]>([]);
  const [displayOrder, setDisplayOrder] = useState<string[]>([]);
  const [busy, setBusy] = useState(false);
  const [progressLabel, setProgressLabel] = useState("Working…");
  const [progressPercent, setProgressPercent] = useState(0);
  const [cancelling, setCancelling] = useState(false);
  // Spectrogram rendering works on files already listed, so the table stays up
  // while it runs; an analysis replaces the list, so it doesn't.
  const [keepResultsWhileBusy, setKeepResultsWhileBusy] = useState(false);
  // Read inside catch blocks to tell "the user cancelled" from a real failure.
  const userCancelled = useRef(false);

  /// `report.files` in display order — the single source of truth for anything
  /// that needs "what the table shows".
  const orderedFiles = useMemo(() => {
    if (!report) return [];
    const byPath = new Map(report.files.map((f) => [f.path, f]));
    return displayOrder
      .map((p) => byPath.get(p))
      .filter((f): f is FolderReport["files"][number] => f != null);
  }, [report, displayOrder]);

  /// `explicitOrder` replaces the display order outright instead of merging
  /// into it — used when re-importing a saved report, whose file order *is* a
  /// display order the user arranged before exporting. Without it the imported
  /// rows would be treated like freshly analyzed ones and re-sorted by path,
  /// silently discarding the arrangement the export was made to preserve.
  const applyReport = useCallback(
    (next: FolderReport, nextTargets: string[], explicitOrder?: string[]) => {
      const present = new Set(next.files.map((f) => f.path));
      setReport(next);
      setTargets(nextTargets);
      setDisplayOrder((prev) => {
        if (explicitOrder) return explicitOrder.filter((p) => present.has(p));
        const kept = prev.filter((p) => present.has(p));
        const known = new Set(kept);
        const fresh = next.files
          .map((f) => f.path)
          .filter((p) => !known.has(p))
          .sort((a, b) => a.localeCompare(b));
        return [...kept, ...fresh];
      });
      onFilesChanged(present);
    },
    [onFilesChanged],
  );

  const startTask = useCallback((label: string, keepResults = false) => {
    userCancelled.current = false;
    setCancelling(false);
    setKeepResultsWhileBusy(keepResults);
    setBusy(true);
    setProgressLabel(label);
    setProgressPercent(0);
  }, []);

  const updateProgress = useCallback((p: Progress, verb: string) => {
    setProgressPercent(p.total > 0 ? Math.round((p.current / p.total) * 100) : 0);
    setProgressLabel(
      p.file ? `${verb} ${p.current + 1}/${p.total} — ${p.file}` : `${verb} ${p.total}/${p.total}`,
    );
  }, []);

  const cancelTask = useCallback(async () => {
    if (!busy) return;
    userCancelled.current = true;
    setCancelling(true);
    setProgressLabel("Cancelling…");
    try {
      await api.cancelTask();
    } catch {
      /* ignore */
    }
  }, [busy]);

  const reset = useCallback(() => {
    setReport(null);
    setTargets([]);
    setDisplayOrder([]);
    onFilesChanged(new Set());
  }, [onFilesChanged]);

  /// Analyze `paths`, adding to the current list rather than replacing it.
  const analyze = useCallback(
    async (paths: string[]) => {
      if (busy || paths.length === 0) return;
      startTask("Analyzing…");
      try {
        const fresh = await api.analyzePaths(paths);
        // Merge: keep everything already listed, append what's new.
        const existing = report?.files ?? [];
        const known = new Set(existing.map((f) => f.path));
        const added = fresh.files.filter((f) => !known.has(f.path));
        const merged: FolderReport = {
          root: report?.root ?? fresh.root,
          files: [...existing, ...added].sort((a, b) => a.path.localeCompare(b.path)),
          has_flac: [...existing, ...added].some((f) => f.flac_md5 != null),
        };
        const nextTargets = [...targets];
        for (const t of paths) if (!nextTargets.includes(t)) nextTargets.push(t);
        applyReport(merged, nextTargets);
        onToast(
          `Added ${added.length} ${added.length === 1 ? "file" : "files"} — ${merged.files.length} in the list.`,
        );
      } catch (e) {
        // Nothing was rendered on cancel or failure (e.g. a dropped file that
        // isn't a supported format) — whatever was on screen before stays.
        if (userCancelled.current || String(e).includes("cancelled")) {
          onToast(
            report && report.files.length > 0
              ? "Analysis cancelled — kept the existing list."
              : "Analysis cancelled.",
          );
        } else {
          onToast(String(e), "error");
        }
      } finally {
        setBusy(false);
        setCancelling(false);
      }
    },
    [busy, report, targets, startTask, applyReport, onToast],
  );

  /// Re-import a previously-saved JSON report without re-analyzing any audio.
  const loadReport = useCallback(
    async (path: string) => {
      if (busy) return;
      startTask("Loading report…");
      try {
        const loaded = await api.loadReport(path);
        const order = loaded.files.map((f) => f.path);
        // Targets come from the report's own paths so "Generate spectrograms"
        // and further drops keep working, same as after a normal folder drop.
        // The same list doubles as the display order — see `applyReport`.
        applyReport(loaded, order, order);
        onToast(
          `Loaded ${loaded.files.length} ${loaded.files.length === 1 ? "file" : "files"} from report.`,
        );
      } catch (e) {
        onToast(String(e), "error");
      } finally {
        setBusy(false);
      }
    },
    [busy, startTask, applyReport, onToast],
  );

  const generateSpectrograms = useCallback(async () => {
    if (busy || targets.length === 0) return;
    startTask("Rendering spectrograms…", true); // keep the table visible
    try {
      const s = await api.generateSpectrograms(targets);
      if (userCancelled.current) {
        onToast(`Spectrograms cancelled — ${s.rendered}/${s.total} rendered.`);
      } else {
        onToast(
          `Rendered ${s.rendered}/${s.total} spectrograms${s.failed ? ` (${s.failed} failed)` : ""}.`,
          s.failed ? "error" : "info",
        );
      }
    } catch (e) {
      if (userCancelled.current || String(e).includes("cancelled")) {
        onToast("Spectrogram generation cancelled.");
      } else {
        onToast(String(e), "error");
      }
    } finally {
      setBusy(false);
      setCancelling(false);
    }
  }, [busy, targets, startTask, onToast]);

  /// Remove one or more rows from the list at once (they are not deleted from
  /// disk). Multiple removals must go through a single `applyReport` call:
  /// calling `removeFile` in a loop reads the same stale `report` closure on
  /// every iteration, so only the last call's result survives `setReport`,
  /// and the display-order "fresh files" logic then mistakes the
  /// already-removed earlier paths for newly-appeared ones and re-appends
  /// them at the bottom instead of dropping them.
  const removeFiles = useCallback(
    (paths: string[]) => {
      if (!report || paths.length === 0) return;
      const toRemove = new Set(paths);
      const files = report.files.filter((f) => !toRemove.has(f.path));
      applyReport(
        { ...report, files, has_flac: files.some((f) => f.flac_md5 != null) },
        targets,
      );
    },
    [report, targets, applyReport],
  );

  const removeFile = useCallback((path: string) => removeFiles([path]), [removeFiles]);

  return {
    report,
    targets,
    displayOrder,
    orderedFiles,
    busy,
    cancelling,
    keepResultsWhileBusy,
    progressLabel,
    progressPercent,
    setDisplayOrder,
    updateProgress,
    analyze,
    loadReport,
    generateSpectrograms,
    cancelTask,
    removeFile,
    removeFiles,
    reset,
  };
}
