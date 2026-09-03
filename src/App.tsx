// Application root: owns the state several features share, and wires the
// pieces together. Anything with real logic of its own lives in a hook or a
// component under src/components — this file should stay readable as a
// description of the app's shape.

import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";

import type { PlaylistFormat } from "./types";
import * as api from "./api";
import { commonDir, fileSearchText, matchesSearch } from "./format";
import { ConfirmDialog } from "./components/ConfirmDialog";
import { ConvertPanel } from "./components/ConvertPanel";
import { Dropzone } from "./components/Dropzone";
import { Footer } from "./components/Footer";
import { DropGuard, Progress } from "./components/Progress";
import { PlaylistFormatModal } from "./components/PlaylistFormatModal";
import { ResultsSummary } from "./components/ResultsSummary";
import { ResultsTable } from "./components/ResultsTable";
import { TagPanel, type TagPanelHandle } from "./components/TagPanel";
import { nextSort, sortFiles, type SortColumn, type SortState } from "./components/tableSort";
import { TopBar } from "./components/TopBar";
import { useAnalysis } from "./components/useAnalysis";
import { useMissingFiles } from "./components/useMissingFiles";
import {
  useConvertProgress,
  useMenuActions,
  useProgressEvents,
  useRevealWindow,
  useSuppressContextMenu,
} from "./components/useAppEvents";
import { useConvertPanel } from "./components/useConvertPanel";
import { useExports } from "./components/useExports";
import { useNativeDrop } from "./components/useNativeDrop";
import { usePlayback } from "./components/usePlayback";
import { usePlaybackQueue } from "./components/usePlaybackQueue";
import {
  useSelection,
  type SelectionModifiers,
} from "./components/useSelection";
import { useRenameFile } from "./components/useRenameFile";
import { useRenumberTracks } from "./components/useRenumberTracks";
import { useGlobalShortcut } from "./components/useGlobalShortcut";
import { useTagCache, useTagPrefetch } from "./components/useTagCache";
import { useToast } from "./components/useToast";
import "./App.css";

export function App() {
  const { toast, showToast } = useToast();
  const tagPanelRef = useRef<TagPanelHandle>(null);
  const [ffmpegAvailable, setFfmpegAvailable] = useState(false);
  const [playlistModalOpen, setPlaylistModalOpen] = useState(false);
  const [searchQuery, setSearchQuery] = useState("");
  const [renumberConfirmOpen, setRenumberConfirmOpen] = useState(false);
  // Which row's name is currently an editable field — the results table's
  // "click twice on the name" rename. `null` for every row rendering its
  // plain name cell as usual.
  const [editingPath, setEditingPath] = useState<string | null>(null);

  const cache = useTagCache();
  const { clear: clearCache, invalidate } = cache;

  // Selection and caches follow the file list: a row that no longer exists
  // can't stay selected, and a cleared list drops its cached tags entirely.
  const [presentPaths, setPresentPaths] = useState<Set<string>>(new Set());
  const onFilesChanged = useCallback(
    (present: Set<string>) => {
      setPresentPaths(present);
      if (present.size === 0) clearCache();
    },
    [clearCache],
  );

  const analysis = useAnalysis({ onToast: showToast, onFilesChanged });
  const orderedPaths = useMemo(
    () => analysis.orderedFiles.map((f) => f.path),
    [analysis.orderedFiles],
  );
  // Whether the listed files are still on disk. Frontend-only and never
  // exported — see useMissingFiles.ts for why it stays out of `FileAnalysis`.
  const presence = useMissingFiles({
    paths: orderedPaths,
    fromReport: analysis.loadedFromReport,
    // A file that comes back needs its tags and cover re-read: the cache
    // recorded the failed read it got while the file was gone, and a cached
    // failure is indistinguishable from "this file has no tags".
    // Passed by reference, not wrapped in an arrow: `invalidate` is already
    // stable, and a fresh arrow here would be one more thing changing every
    // render for no reason.
    onReappeared: cache.invalidate,
    onToast: showToast,
  });


  // Search box in the top bar: a pure display filter over the table. The
  // selection, drag-reorder and every export read from
  // `analysis.orderedFiles`/`orderedPaths` directly and never learn this
  // filter exists, so hiding a row here can't drop it from a CSV/JSON/M3U
  // export. Playback is the one exception — see `displayedPaths` below.
  //
  // Matches against everything the row actually shows (`fileSearchText`), not
  // just the file name — so "24-bit", "transcoded" or "flac" filters the list
  // too. `matchesSearch` does whole-word matching, not a plain substring
  // search, so a closed word like "16-" (typed for "16-bit") can't match
  // "160 MB" or "116 MB" just because they contain "16".
  const hasSearchQuery = searchQuery.trim().length > 0;
  const visiblePaths = useMemo(() => {
    if (!hasSearchQuery) return null;
    const matches = new Set<string>();
    for (const f of analysis.orderedFiles) {
      if (matchesSearch(fileSearchText(f), searchQuery)) matches.add(f.path);
    }
    return matches;
  }, [analysis.orderedFiles, hasSearchQuery, searchQuery]);

  // Column sorting (ResultsTable's headers) — state lives here, not in
  // ResultsTable, for the same reason `displayedPaths` below exists: nothing
  // sorted by default (`null` = the natural/drag order the table opens in).
  const [sort, setSort] = useState<SortState | null>(null);
  const onSortChange = useCallback((column: SortColumn) => {
    setSort((s) => nextSort(s, column));
  }, []);

  // What the table is actually showing right now — filtered by the search
  // box, then sorted if a column sort is active. Unlike `visiblePaths`
  // above, this *does* drive playback: the footer's Play button starting
  // "the wrong track" (the first one in import order, not the first one on
  // screen) is confusing in a way dropping a hidden row from a CSV export
  // never would be, so Play/Previous/Next (usePlaybackQueue) and the natural
  // end-of-track auto-advance (usePlayback) both walk this list instead of
  // the raw `orderedPaths`. Exports and drag-reorder are unaffected — they
  // never read this.
  const displayedFiles = useMemo(() => {
    const filtered =
      visiblePaths == null
        ? analysis.orderedFiles
        : analysis.orderedFiles.filter((f) => visiblePaths.has(f.path));
    return sort ? sortFiles(filtered, sort) : filtered;
  }, [analysis.orderedFiles, visiblePaths, sort]);
  const displayedPaths = useMemo(() => displayedFiles.map((f) => f.path), [displayedFiles]);

  const selection = useSelection(orderedPaths);
  const { pruneSelection } = selection;
  useEffect(() => pruneSelection(presentPaths), [presentPaths, pruneSelection]);

  /// Point the listing at a folder the files moved to.
  ///
  /// Only the *missing* rows are repointed, and only those whose file name
  /// turns up under the chosen folder — so a listing assembled from several
  /// places (a mixtape, a comparison set) can be fixed one folder at a time
  /// instead of all-or-nothing.
  const relocateMissing = useCallback(async () => {
    const missing = [...presence.missing];
    if (missing.length === 0) return;
    const root = await open({ directory: true, multiple: false, title: "Where did these files move to?" });
    if (typeof root !== "string") return;
    try {
      const moves = await api.relocatePaths(missing, root);
      if (moves.length === 0) {
        showToast("No matching file names under that folder.", "error");
        return;
      }
      analysis.relocateFiles(moves);
      // The selected rows are keyed by path, and their paths just changed, so
      // the selection now points at rows that no longer exist. Dropping it
      // outright — rather than remapping it — also closes the tag panel,
      // which would otherwise sit open over files it can no longer write to.
      selection.clearSelection();
      // The rows now point somewhere new: their tags and covers were never
      // read from there, and the presence check is about paths that no longer
      // exist in the listing.
      cache.invalidate(moves.map((m) => m.to));
      void presence.refresh();
      const rest = missing.length - moves.length;
      showToast(
        rest === 0
          ? `Relocated all ${moves.length} file${moves.length === 1 ? "" : "s"}.`
          : `Relocated ${moves.length} of ${missing.length} — ${rest} still missing.`,
      );
    } catch (e) {
      showToast(String(e), "error");
    }
  }, [presence, analysis, cache, selection, showToast]);

  /// Write the repointed listing back over the report it was loaded from.
  ///
  /// Separate from "Save…", which asks for a destination: after a relocation
  /// the useful action is to update the file you just opened, and being sent
  /// to a file picker to retype its own name is friction with no upside. Only
  /// the JSON is rewritten — the CSV beside it, if any, describes the same
  /// analysis and is regenerated by Save… when the user wants both.
  const updateLoadedReport = useCallback(async () => {
    const dest = analysis.reportPath;
    if (!dest || !analysis.report) return;
    try {
      await api.saveReportJson(dest, { ...analysis.report, files: analysis.orderedFiles });
      showToast("Report updated.");
    } catch (e) {
      showToast(String(e), "error");
    }
  }, [analysis.reportPath, analysis.report, analysis.orderedFiles, showToast]);

  // A row click, Select all, or Deselect all can each discard an unsaved tag
  // edit — see `useTagEditor`'s selection-key reset, which runs during
  // render, before there's any chance to step in afterwards. So the
  // confirmation has to happen here, *before* the selection actually
  // changes, not in the tag panel. `pendingAction` holds whatever selection
  // change is waiting on that confirmation — a plain callback rather than a
  // `{path, ev}` pair, so the same guard covers all three without any of
  // them needing a shape it doesn't have. The tag panel's own close button
  // (`onClose` below) goes through `guardedDeselectAll` too — it used to
  // clear the selection unconditionally, silently dropping whatever was
  // half-edited, which was a bug rather than a deliberate exception.
  const [pendingAction, setPendingAction] = useState<(() => void) | null>(null);
  const guardedSelectRow = useCallback(
    (path: string, ev: SelectionModifiers) => {
      const noopReselect =
        !ev.shiftKey &&
        !ev.metaKey &&
        !ev.ctrlKey &&
        selection.selectedPaths.length === 1 &&
        selection.selectedPaths[0] === path;
      const run = () => selection.selectRow(path, ev);
      if (!noopReselect && tagPanelRef.current?.isDirty()) {
        setPendingAction(() => run);
        return;
      }
      run();
    },
    [selection],
  );
  const guardedSelectAll = useCallback(() => {
    if (orderedPaths.length === 0) return;
    const run = () => selection.selectAll();
    if (tagPanelRef.current?.isDirty()) {
      setPendingAction(() => run);
      return;
    }
    run();
  }, [selection, orderedPaths]);
  const guardedDeselectAll = useCallback(() => {
    if (selection.selectedPaths.length === 0) return;
    const run = () => selection.clearSelection();
    if (tagPanelRef.current?.isDirty()) {
      setPendingAction(() => run);
      return;
    }
    run();
  }, [selection]);
  const guardedInvertSelection = useCallback(() => {
    if (orderedPaths.length === 0) return;
    const run = () => selection.invertSelection();
    if (tagPanelRef.current?.isDirty()) {
      setPendingAction(() => run);
      return;
    }
    run();
  }, [selection, orderedPaths]);
  // Opening the rename field doesn't change the selection, but it does mean
  // the row's identity (its path) is about to change under the tag panel —
  // same discard risk as actually reselecting, so it goes through the same
  // guard.
  const guardedStartRename = useCallback(
    (path: string) => {
      const run = () => setEditingPath(path);
      if (tagPanelRef.current?.isDirty()) {
        setPendingAction(() => run);
        return;
      }
      run();
    },
    [],
  );
  const cancelRename = useCallback(() => setEditingPath(null), []);
  const confirmPendingAction = useCallback(() => {
    if (pendingAction) {
      tagPanelRef.current?.discardEdits();
      pendingAction();
    }
    setPendingAction(null);
  }, [pendingAction]);

  // `displayedPaths`, not `orderedPaths`: both the natural end-of-track
  // auto-advance and the footer's transport buttons should walk the table
  // exactly as it's currently shown — filtered and sorted — not the raw
  // import order. See `displayedPaths`'s own doc comment above.
  //
  // Playback rules (see `usePlayback`'s own doc comment for the full
  // rationale): no selection plays the whole table from the top; one row
  // selected starts there and still plays on to the end; several rows
  // selected play just that selection, in table order, and stop once it's
  // exhausted. Whichever applies is decided once, when Play starts a session
  // (`usePlayback`'s `activeQueue`) — changing the selection while a track is
  // already playing has no effect on that session, only on the next one.
  const playback = usePlayback(displayedPaths, selection.selectedPaths, showToast);
  useTagPrefetch(orderedPaths, cache.fetchMissing);

  // The conversion panel's own session — entirely separate from `analysis`
  // (its imports never touch the results table). Pausing playback before a
  // batch starts is this app's job, not the hook's: `useConvertPanel` has no
  // idea whether anything is playing, so it just calls back here.
  //
  // The pause itself isn't just cosmetic: a batch runs the CPU-heavy encoders
  // in parallel across cores (see `commands::batch::parallel_map_ordered`),
  // and the audio callback thread competing for the same cores is a real way
  // to get dropouts — pausing avoids that rather than hoping it doesn't
  // happen. It's remembered in a ref (not state — nothing needs to re-render
  // off it) so it can be resumed the moment the freeze lifts, rather than
  // leaving the track paused for no reason once the batch is done.
  const resumeAfterConvertRef = useRef(false);
  const convert = useConvertPanel({
    onToast: showToast,
    onBeforeStart: () => {
      resumeAfterConvertRef.current = playback.nowPlaying !== null && !playback.paused;
      if (resumeAfterConvertRef.current) {
        playback.togglePause();
        showToast("Playback paused for the conversion");
      }
    },
  });
  useConvertProgress(convert.updateProgress);

  // Fires once per batch, on the busy -> idle transition (success, failure,
  // or cancel all end up here the same way, via `useConvertPanel`'s own
  // `finally`) — the counterpart to `onBeforeStart` above.
  const wasConvertBusy = useRef(convert.busy);
  useEffect(() => {
    if (wasConvertBusy.current && !convert.busy && resumeAfterConvertRef.current) {
      playback.togglePause();
      resumeAfterConvertRef.current = false;
      showToast("Playback resumed");
    }
    wasConvertBusy.current = convert.busy;
  }, [convert.busy, playback, showToast]);

  // What the footer's transport buttons show/trigger — see the hook's own
  // doc comment for why it reads `activeQueue` rather than recomputing.
  const playbackQueue = usePlaybackQueue({
    orderedPaths: displayedPaths,
    selectedPaths: selection.selectedPaths,
    nowPlaying: playback.nowPlaying,
    activeQueue: playback.activeQueue,
    play: playback.play,
    togglePause: playback.togglePause,
    stepQueue: playback.stepQueue,
  });

  const exports = useExports({
    report: analysis.report,
    orderedFiles: analysis.orderedFiles,
    targets: analysis.targets,
    busy: analysis.busy,
    tags: cache.tags,
    onToast: showToast,
  });

  const drop = useNativeDrop({
    busy: analysis.busy,
    converting: convert.busy,
    tagPanelRef,
    onAnalyze: analysis.analyze,
    onLoadReport: analysis.loadReport,
    onImportForConvert: convert.addTargets,
    onToast: showToast,
  });

  useProgressEvents(analysis.updateProgress);
  useRevealWindow();
  useSuppressContextMenu();

  useEffect(() => {
    api
      .ffmpegAvailable()
      .then(setFfmpegAvailable)
      .catch(() => setFfmpegAvailable(false));
  }, []);

  const menuActions = useMemo(
    () => ({
      exportM3u: () => void exports.exportPlaylist("Simple"),
      exportM3uExtended: () => void exports.exportPlaylist("Extended"),
      exportCsv: () => void exports.exportReport("csv"),
      exportJson: () => void exports.exportReport("json"),
      reset: () => {
        playback.stop();
        analysis.reset();
        setSearchQuery("");
      },
      generateSpectrograms: () => void analysis.generateSpectrograms(),
    }),
    [exports, analysis, playback],
  );
  useMenuActions(menuActions);

  const pickFolder = useCallback(async () => {
    const dir = await open({ directory: true, multiple: false });
    if (typeof dir === "string") await analysis.analyze([dir]);
  }, [analysis]);

  const deleteSelected = useCallback(() => {
    if (selection.selectedPaths.length === 0) return;
    selection.selectedPaths.forEach((path) => playback.stopIfPlaying(path));
    // One call for every path, not a loop of single removals: `removeFile`
    // in a loop would read the same stale report on each iteration and lose
    // all but the last removal (see `removeFiles`'s doc comment).
    analysis.removeFiles(selection.selectedPaths);
    selection.clearSelection();
  }, [selection, playback, analysis]);

  const deleteRow = useCallback(
    (path: string, isSelected: boolean) => {
      // If the row is part of a multi-selection, delete all selected rows.
      // Otherwise, delete only this row.
      if (isSelected && selection.selectedPaths.length > 1) {
        deleteSelected();
      } else {
        playback.stopIfPlaying(path);
        analysis.removeFile(path);
      }
    },
    [playback, analysis, selection.selectedPaths, deleteSelected],
  );

  // DEL (or Backspace on Mac) removes all selected rows.
  useGlobalShortcut((ev) => {
    if (ev.key !== "Delete" && ev.key !== "Backspace") return;
    if (selection.selectedPaths.length === 0) return;
    ev.preventDefault();
    deleteSelected();
  });

  // Up/Down move the selection one row through the *displayed* order, so the
  // keyboard follows the search filter and any manual drag-reorder rather
  // than some underlying list the user cannot see. Shift extends instead of
  // replacing, matching what Shift+click already does.
  //
  // With nothing selected yet, Down starts at the top and Up at the bottom —
  // the same convention as a native list, and more useful than doing nothing.
  useGlobalShortcut((ev) => {
    if (ev.key !== "ArrowDown" && ev.key !== "ArrowUp") return;
    const paths = displayedPaths;
    if (paths.length === 0) return;
    ev.preventDefault();

    const down = ev.key === "ArrowDown";
    const current = selection.selectedPaths;
    // The row to move *from* is the last one selected, so extending a
    // selection downward and then reversing behaves the way a list should.
    const from = current.length > 0 ? paths.indexOf(current[current.length - 1]) : -1;
    const next =
      from === -1
        ? down
          ? 0
          : paths.length - 1
        : Math.min(paths.length - 1, Math.max(0, from + (down ? 1 : -1)));
    const path = paths[next];
    if (path == null || path === current[current.length - 1]) return;

    guardedSelectRow(path, {
      shiftKey: ev.shiftKey,
      metaKey: false,
      ctrlKey: false,
    });
    // Keep the row on screen. Reaching into the DOM is the exception the
    // frontend rules allow for real geometry work: scroll position is not
    // state React renders, and `block: "nearest"` only moves the list when
    // the row is actually out of view, so holding an arrow key scrolls
    // steadily instead of recentring on every step.
    document
      .querySelector(`tr[data-path="${CSS.escape(path)}"]`)
      ?.scrollIntoView({ block: "nearest" });
  });

  // Ctrl/Cmd+A selects every track; Ctrl/Cmd+Shift+A — there's no single
  // conventional shortcut for "deselect all", but Shift reversing a
  // shortcut's meaning is a common enough pattern (undo/redo, browsers'
  // reopen-tab) to read as "the opposite of Select all" rather than an
  // arbitrary binding. Both go through the same guards as their toolbar
  // buttons (`guardedSelectAll`/`guardedDeselectAll`), so an unsaved tag
  // edit gets the same "discard changes?" prompt from the keyboard as it
  // does from a click.
  useGlobalShortcut((ev) => {
    if (!(ev.metaKey || ev.ctrlKey) || ev.key.toLowerCase() !== "a") return;
    ev.preventDefault();
    if (ev.shiftKey) guardedDeselectAll();
    else guardedSelectAll();
  });

  const onPlaylistConfirm = useCallback(
    (format: PlaylistFormat) => {
      setPlaylistModalOpen(false);
      void exports.exportPlaylist(format);
    },
    [exports],
  );

  const hasResults =
    analysis.report != null && analysis.orderedFiles.length > 0;
  // Memoized, not just computed: this array feeds the tag panel's derived
  // state (distinct covers, extended rows), and a fresh identity on every
  // render would keep resetting the cover carousel to the first image.
  const { tagSetsFor } = cache;
  const selectedTagSets = useMemo(
    () => tagSetsFor(selection.selectedPaths),
    [tagSetsFor, selection.selectedPaths],
  );
  // Every distinct format in the selection, for the extended-tags pop-in's
  // format indicator — extended tags (and what its "+" picker can offer)
  // depend on the format, the same reason Mp3tag shows it too.
  const selectedFormats = useMemo(() => {
    const paths = new Set(selection.selectedPaths);
    const formats = new Set(
      analysis.orderedFiles.filter((f) => paths.has(f.path)).map((f) => f.format),
    );
    return [...formats];
  }, [analysis.orderedFiles, selection.selectedPaths]);

  // `selection.selectedPaths` is in click order (see useSelection's doc
  // comment), not display order — renumbering has to follow the table's
  // order instead, or the assigned track numbers wouldn't match what the
  // user sees top to bottom.
  const selectedPathSet = useMemo(
    () => new Set(selection.selectedPaths),
    [selection.selectedPaths],
  );
  const orderedSelectedPaths = useMemo(
    () => analysis.orderedFiles.filter((f) => selectedPathSet.has(f.path)).map((f) => f.path),
    [analysis.orderedFiles, selectedPathSet],
  );

  const renumberTracks = useRenumberTracks({ onSaved: invalidate, onToast: showToast });
  const confirmRenumber = useCallback(() => {
    setRenumberConfirmOpen(false);
    void renumberTracks.renumber(orderedSelectedPaths);
  }, [renumberTracks, orderedSelectedPaths]);

  // Everything that has to follow a file's identity changing: the row itself
  // (path + file_name, via `analysis.renameFile`), the selection (so the tag
  // panel stays open on the same file rather than reading as deselected —
  // see `replacePath`'s doc comment), and the tag cache, which has nothing
  // filed under the new path yet. Order doesn't matter for correctness here
  // (React 19 batches all three into one re-render), only that all three
  // happen together.
  const { rename: commitRename, busy: renameBusy } = useRenameFile({
    onRenamed: (oldPath, newPath, newFileName) => {
      analysis.renameFile(oldPath, newPath, newFileName);
      selection.replacePath(oldPath, newPath);
      invalidate([newPath]);
      setEditingPath(null);
    },
    onToast: showToast,
  });
  const submitRename = useCallback(
    (path: string, newStem: string) => {
      // A rename mid-playback would leave `nowPlaying` pointing at a path
      // that no longer exists — sidestepped by just stopping first, the same
      // way deleting the currently-playing row already does.
      playback.stopIfPlaying(path);
      void commitRename(path, newStem);
    },
    [playback, commitRename],
  );

  return (
    <div id="app">
      <TopBar
        busy={analysis.busy}
        hasReport={hasResults}
        canGenerateSpectrograms={analysis.targets.length > 0}
        ffmpegAvailable={ffmpegAvailable}
        searchQuery={searchQuery}
        onSearchChange={setSearchQuery}
        selectedCount={selection.selectedPaths.length}
        onSelectAll={guardedSelectAll}
        onDeselectAll={guardedDeselectAll}
        onInvertSelection={guardedInvertSelection}
        renumberBusy={renumberTracks.busy}
        onRenumberTracks={() => setRenumberConfirmOpen(true)}
        onPick={() => void pickFolder()}
        onSave={() => void exports.saveReport()}
        onExportPlaylist={() => setPlaylistModalOpen(true)}
        onGenerateSpectrograms={() => void analysis.generateSpectrograms()}
        onReset={menuActions.reset}
        onOpenConvert={convert.togglePanel}
        onRefreshPresence={() => void presence.refresh()}
        checkingPresence={presence.checking}
        missingCount={presence.missing.size}
        onRelocateMissing={() => void relocateMissing()}
        canUpdateReport={analysis.reportPath != null}
        onUpdateReport={() => void updateLoadedReport()}
      />

      <div className="main-row">
        {selection.selectedPaths.length > 0 && (
          <TagPanel
            ref={tagPanelRef}
            selectedPaths={selection.selectedPaths}
            tagSets={selectedTagSets}
            formats={selectedFormats}
            coverDragOver={drop.overCover}
            onClose={guardedDeselectAll}
            onSaved={invalidate}
            onToast={showToast}
          />
        )}

        <div className="main-col">
          {!hasResults && !analysis.busy && (
            <Dropzone dragOver={drop.overList} />
          )}

          {hasResults &&
            (!analysis.busy || analysis.keepResultsWhileBusy) &&
            analysis.report && (
              <section className="results">
                <ResultsSummary
                  report={analysis.report}
                  rootPath={commonDir(analysis.report.files.map((f) => f.path))}
                  selectedCount={selection.selectedPaths.length}
                  visibleCount={visiblePaths == null ? null : visiblePaths.size}
                  missing={presence.missing}
                  onToast={showToast}
                />
                <ResultsTable
                  files={analysis.orderedFiles}
                  covers={cache.covers}
                  tags={cache.tags}
                  nowPlaying={playback.nowPlaying}
                  selectedPaths={selection.selectedPaths}
                  visiblePaths={visiblePaths}
                  sort={sort}
                  onSortChange={onSortChange}
                  editingPath={editingPath}
                  renameBusy={renameBusy}
                  missing={presence.missing}
                  onSelectRow={guardedSelectRow}
                  onStartRename={guardedStartRename}
                  onCancelRename={cancelRename}
                  onSubmitRename={submitRename}
                  onReorder={analysis.setDisplayOrder}
                  onReveal={(p) =>
                    api
                      .revealInFolder(p)
                      .catch((e) => showToast(String(e), "error"))
                  }
                  onTogglePlay={playback.togglePlay}
                  onDelete={deleteRow}
                />
                <Footer
                  files={analysis.orderedFiles}
                  selectedPaths={selection.selectedPaths}
                  nowPlaying={playback.nowPlaying}
                  paused={playback.paused}
                  position={playback.position}
                  volume={playback.volume}
                  muted={playback.muted}
                  canGoPrevious={playbackQueue.canGoPrevious}
                  canGoNext={playbackQueue.canGoNext}
                  onPrevious={playbackQueue.onPrevious}
                  onTogglePause={playbackQueue.onPlayPause}
                  onNext={playbackQueue.onNext}
                  onSeek={playback.seek}
                  onToggleMute={playback.toggleMute}
                  onVolumeChange={playback.setVolume}
                />
              </section>
            )}

          {analysis.busy && (
            <Progress
              label={analysis.progressLabel}
              percent={analysis.progressPercent}
              cancelDisabled={analysis.cancelling}
              onCancel={() => void analysis.cancelTask()}
            />
          )}
        </div>

        {convert.open && (
          <ConvertPanel
            targets={convert.targets}
            selected={convert.selected}
            format={convert.format}
            bitrateKbps={convert.bitrateKbps}
            flacEffort={convert.flacEffort}
            preserveModtime={convert.preserveModtime}
            copyOthers={convert.copyOthers}
            importing={convert.importing}
            busy={convert.busy}
            cancelling={convert.cancelling}
            progressLabel={convert.progressLabel}
            dragOver={drop.overConvert}
            mainSelectionCount={selection.selectedPaths.length}
            onClose={convert.closePanel}
            onSetFormat={convert.setFormat}
            onSetBitrateKbps={convert.setBitrateKbps}
            onSetFlacEffort={convert.setFlacEffort}
            onSetPreserveModtime={convert.setPreserveModtime}
            onSetCopyOthers={convert.setCopyOthers}
            onRemoveTarget={convert.removeTarget}
            onClearTargets={convert.clearTargets}
            onToggleSelected={convert.toggleSelected}
            onAddSelected={() => void convert.addTargets(selection.selectedPaths)}
            onConvert={() => void convert.convert()}
            onCancel={() => void convert.cancel()}
          />
        )}
      </div>

      {/* Blocks every control app-wide while a conversion runs — "the whole
          app should freeze" was explicit in the request, so this is a plain
          click-blocking overlay rather than threading a `disabled` prop
          through the toolbar, the table and the footer individually. Sits
          below the conversion panel itself (see z-index in App.css), which
          stays interactive for its own Cancel button. */}
      {convert.busy && <div className="app-freeze-overlay" />}

      {drop.blocked && (
        <DropGuard
          message={
            convert.busy
              ? "Conversion in progress — please wait before dropping files"
              : "Analysis in progress — please wait before dropping files"
          }
        />
      )}

      <PlaylistFormatModal
        open={playlistModalOpen}
        onClose={() => setPlaylistModalOpen(false)}
        onConfirm={onPlaylistConfirm}
      />

      <ConfirmDialog
        open={pendingAction != null}
        title="Discard changes?"
        message="This track has unsaved tag changes. Selecting another one will discard them."
        confirmLabel="Discard"
        danger
        onConfirm={confirmPendingAction}
        onCancel={() => setPendingAction(null)}
      />

      <ConfirmDialog
        open={renumberConfirmOpen}
        title="Renumber tracks?"
        message={`Sets Track to 1–${orderedSelectedPaths.length} and Track Total to ${orderedSelectedPaths.length} for the ${orderedSelectedPaths.length} selected tracks, in the order shown in the table — overwriting any existing values.`}
        confirmLabel="Renumber"
        onConfirm={confirmRenumber}
        onCancel={() => setRenumberConfirmOpen(false)}
      />

      {toast && <div className={`toast ${toast.kind}`}>{toast.msg}</div>}
    </div>
  );
}
