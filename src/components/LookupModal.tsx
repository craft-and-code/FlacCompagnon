// Online lookup pop-in (MusicBrainz + Discogs). Only ever runs on the tag
// panel's explicit "Search online" click, never automatically.
//
// Nothing here writes a file: applying a result stages it into the tag panel's
// own fields and cover, exactly like typing them by hand, so the tag panel's
// Save button stays the only thing that ever touches a file — and its Reset
// still discards a staged lookup like any other unsaved edit.

import { useCallback, useEffect, useState } from "react";

import type { LookupCandidate, LookupRelease } from "../types";
import { Modal } from "./Modal";
import { LookupDetailView } from "./LookupDetailView";
import { LookupSearchView } from "./LookupSearchView";
import { useLookup } from "./useLookup";

// The Discogs personal access token lives in localStorage (nowhere on the Rust
// side) since it's the user's own credential, entered once and reused across
// sessions. Its absence just means Discogs is skipped, not an error.
const DISCOGS_TOKEN_KEY = "flaccompagnon.discogsToken";

export interface LookupModalProps {
  open: boolean;
  onClose: () => void;
  /// Paths in the current selection — drives the single-file/multi-file rules
  /// and the file-name fallback query.
  selectedPaths: string[];
  /// A MusicBrainz Release ID every tagged file in the selection agrees on, if
  /// any (e.g. left by Picard) — lets the fuzzy text search be skipped.
  existingReleaseId: string | null;
  /// Current tag panel values, used to prefill the query.
  prefill: { artist: string; album: string };
  onApply: (release: LookupRelease, trackIndex: number | null) => void;
  onToast: (msg: string, kind?: "info" | "error") => void;
}

// Best-effort query guess from a file name when there are no usable tags to
// prefill from — strips a leading track number and, when the name looks like
// "Artist - Title" (the most common rip naming), splits on the dash so both
// halves end up in the query. Never applied silently: it only prefills the
// box, the user still has to confirm.
function guessQueryFromFilename(path: string): string {
  const base = path.split(/[\\/]/).pop() ?? path;
  const stem = base.replace(/\.[A-Za-z0-9]+$/, "");
  const cleaned = stem.replace(/^\d{1,3}[.\-_\s]+/, "").trim();
  const m = cleaned.match(/^(.+?)\s*[-–—]\s*(.+)$/);
  return m ? `${m[1].trim()} ${m[2].trim()}`.trim() : cleaned;
}

export function LookupModal({
  open,
  onClose,
  selectedPaths,
  existingReleaseId,
  prefill,
  onApply,
  onToast,
}: LookupModalProps) {
  const [query, setQuery] = useState("");
  const [discogsToken, setDiscogsToken] = useState(
    () => localStorage.getItem(DISCOGS_TOKEN_KEY) ?? "",
  );
  const [matchedByExistingId, setMatchedByExistingId] = useState(false);
  const [selectedTrackIndex, setSelectedTrackIndex] = useState<number | null>(null);

  const lookup = useLookup(discogsToken);
  const { reset, search, selectCandidate, setStatus, clearDetail } = lookup;

  const runSearch = useCallback(
    async (q: string) => {
      const partialFailure = await search(q);
      if (partialFailure) onToast(partialFailure, "error");
    },
    [search, onToast],
  );

  // Prefill from whatever's already in the fields (whether it came from the
  // files' own tags or a previous lookup) so the common case is just "open,
  // hit Enter". With no tags at all, fall back to a guess from the file name —
  // shown but not auto-searched, since it's a guess rather than a tag value.
  const prefillAndMaybeSearch = useCallback(() => {
    const fromTags = [prefill.artist.trim(), prefill.album.trim()].filter(Boolean).join(" ");
    if (fromTags) {
      setQuery(fromTags);
      void runSearch(fromTags);
      return;
    }
    const guessed = selectedPaths[0] ? guessQueryFromFilename(selectedPaths[0]) : "";
    setQuery(guessed);
    if (guessed) setStatus("Guessed from the file name — check it before searching.");
  }, [prefill.artist, prefill.album, selectedPaths, runSearch, setStatus]);

  // Opening starts a fresh session. Deliberately keyed on `open` alone: the
  // prefill sources are read once at open time, so editing the tag fields
  // behind an open pop-in can't retrigger a search.
  useEffect(() => {
    if (!open) return;
    reset();
    setMatchedByExistingId(false);
    setSelectedTrackIndex(null);

    // Already tagged with a MusicBrainz Release ID? Skip the text search and
    // go straight to that exact release, falling back to a normal search if
    // the tagged ID turns out to be stale.
    if (existingReleaseId) {
      setMatchedByExistingId(true);
      void selectCandidate({
        source: "MusicBrainz",
        id: existingReleaseId,
        title: "",
        artist: "",
        year: null,
        track_count: null,
      }).then((ok) => {
        if (!ok) {
          setMatchedByExistingId(false);
          prefillAndMaybeSearch();
        }
      });
      return;
    }
    prefillAndMaybeSearch();
  }, [open]);

  const onPick = (candidate: LookupCandidate) => {
    setSelectedTrackIndex(null);
    setMatchedByExistingId(false);
    void selectCandidate(candidate);
  };

  const onBack = () => {
    clearDetail();
    // Coming back from the "matched via existing ID" shortcut, the search box
    // was never filled in — do it now so there's something to search manually.
    if (matchedByExistingId && !query.trim()) {
      setMatchedByExistingId(false);
      prefillAndMaybeSearch();
    }
  };

  const onTokenChange = (token: string) => {
    const trimmed = token.trim();
    setDiscogsToken(trimmed);
    localStorage.setItem(DISCOGS_TOKEN_KEY, trimmed);
  };

  return (
    <Modal open={open} onClose={onClose} innerClassName="lookup-inner" title="Search online">
      {lookup.loading.on && (
        <div className="lookup-loading">
          <span className="spinner" />
          <span>{lookup.loading.label}</span>
        </div>
      )}

      {lookup.detail ? (
        <LookupDetailView
          release={lookup.detail}
          selectionCount={selectedPaths.length}
          matchedByExistingId={matchedByExistingId}
          selectedTrackIndex={selectedTrackIndex}
          onSelectTrack={setSelectedTrackIndex}
          onBack={onBack}
          onApply={() => {
            const release = lookup.detail;
            if (!release) return;
            onApply(release, selectedPaths.length === 1 ? selectedTrackIndex : null);
            onClose();
            onToast("Online result staged into the tag panel — review it, then Save.");
          }}
        />
      ) : (
        <LookupSearchView
          query={query}
          onQueryChange={setQuery}
          onSubmit={() => void runSearch(query)}
          searching={lookup.searching}
          discogsToken={discogsToken}
          onDiscogsTokenChange={onTokenChange}
          status={lookup.status}
          candidates={lookup.candidates}
          onPick={onPick}
        />
      )}
    </Modal>
  );
}
