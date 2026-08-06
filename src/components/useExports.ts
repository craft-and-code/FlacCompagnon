// Everything that writes a file the user picked a location for: the CSV/JSON
// report and the M3U playlist.
//
// Report saving is two backend commands rather than one that always writes
// both, so the menu's standalone "Export CSV"/"Export JSON" can produce a
// single file — the toolbar's "Save…" just calls both in sequence instead of
// the backend carrying a third do-both command that would duplicate them.

import { useCallback } from "react";
import { save } from "@tauri-apps/plugin-dialog";

import type { FileAnalysis, FolderReport, PlaylistEntry, PlaylistFormat, TagSet } from "../types";
import * as api from "../api";
import { commonDir, playlistNameFrom, reportNameFrom } from "../format";

export interface UseExportsArgs {
  report: FolderReport | null;
  /// The table's display order — what every export follows.
  orderedFiles: FileAnalysis[];
  targets: string[];
  busy: boolean;
  tags: Map<string, TagSet | null>;
  onToast: (msg: string, kind?: "info" | "error") => void;
}

export function useExports({
  report,
  orderedFiles,
  targets,
  busy,
  tags,
  onToast,
}: UseExportsArgs) {
  /// The report as it should be written: display order, not the backend's own
  /// path-sorted order. `null` means "nothing to save", a no-op for callers.
  const reportToSave = useCallback((): FolderReport | null => {
    if (busy || !report) return null;
    return { ...report, files: orderedFiles };
  }, [busy, report, orderedFiles]);

  const defaultPath = useCallback(
    (name: string): string => {
      const dir = report ? commonDir(report.files.map((f) => f.path)) : "";
      if (!dir) return name;
      const sep = dir.includes("\\") ? "\\" : "/";
      return dir.endsWith(sep) ? `${dir}${name}` : `${dir}${sep}${name}`;
    },
    [report],
  );

  const nameSource = targets.length === 1 ? targets[0] : (report?.root ?? "");

  /// One dialog, both formats — the extension of what you pick only decides
  /// the stem, since each command forces its own.
  const saveReport = useCallback(async () => {
    const payload = reportToSave();
    if (!payload) return;
    const dest = await save({
      defaultPath: defaultPath(reportNameFrom(nameSource, "csv")),
      filters: [{ name: "Report (CSV + JSON)", extensions: ["csv", "json"] }],
    });
    if (typeof dest !== "string") return;
    try {
      await api.saveReportCsv(dest, payload);
      await api.saveReportJson(dest, payload);
      onToast("Saved (CSV + JSON).");
    } catch (e) {
      onToast(String(e), "error");
    }
  }, [reportToSave, defaultPath, nameSource, onToast]);

  const exportReport = useCallback(
    async (format: "csv" | "json") => {
      const payload = reportToSave();
      if (!payload) return;
      const dest = await save({
        defaultPath: defaultPath(reportNameFrom(nameSource, format)),
        filters: [{ name: `${format.toUpperCase()} report`, extensions: [format] }],
      });
      if (typeof dest !== "string") return;
      try {
        if (format === "csv") await api.saveReportCsv(dest, payload);
        else await api.saveReportJson(dest, payload);
        onToast(`Saved (${format.toUpperCase()}).`);
      } catch (e) {
        onToast(String(e), "error");
      }
    },
    [reportToSave, defaultPath, nameSource, onToast],
  );

  /// Built entirely from data already on hand — duration from the analysis,
  /// title/artist from the tag cache — falling back to the plain file name for
  /// anything untagged, same as any player would.
  const exportPlaylist = useCallback(
    async (format: PlaylistFormat) => {
      if (busy || !report) return;
      const ext = format === "Extended" ? "m3u8" : "m3u";
      const dest = await save({
        defaultPath: defaultPath(playlistNameFrom(nameSource, ext)),
        filters: [
          {
            name: format === "Extended" ? "Extended M3U" : "Simple M3U",
            extensions: [ext],
          },
        ],
      });
      if (typeof dest !== "string") return;
      try {
        const entries: PlaylistEntry[] = orderedFiles.map((f) => {
          const t = tags.get(f.path);
          return {
            path: f.path,
            duration_secs: f.duration_secs,
            title: t?.title ?? null,
            artist: t?.artist ?? null,
          };
        });
        await api.savePlaylist(dest, entries, format);
        onToast(`Playlist saved (${entries.length} track${entries.length === 1 ? "" : "s"}).`);
      } catch (e) {
        onToast(String(e), "error");
      }
    },
    [busy, report, orderedFiles, tags, defaultPath, nameSource, onToast],
  );

  return { saveReport, exportReport, exportPlaylist };
}
