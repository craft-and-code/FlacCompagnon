// The online lookup "session": provider calls, candidate list, release detail,
// and the status/loading the pop-in shows around them.
//
// Every fetch is guarded by a request identity rather than an AbortController
// because the backend calls can't actually be cancelled — the guard just makes
// a superseded response (a newer search, or the pop-in being reopened) render
// nothing instead of overwriting fresher results.

import { useCallback, useRef, useState } from "react";

import type { LookupCandidate, LookupRelease } from "../types";
import * as api from "../api";
import { LatestRequest } from "./latestRequest";

export interface LookupStatus {
  msg: string;
  kind: "info" | "error";
}

export interface LookupLoading {
  on: boolean;
  label: string;
}

export function useLookup(discogsToken: string) {
  const [candidates, setCandidates] = useState<LookupCandidate[]>([]);
  const [detail, setDetail] = useState<LookupRelease | null>(null);
  const [status, setStatusState] = useState<LookupStatus>({ msg: "", kind: "info" });
  const [loading, setLoading] = useState<LookupLoading>({ on: false, label: "" });
  const [searching, setSearching] = useState(false);
  const requests = useRef(new LatestRequest());

  const setStatus = useCallback((msg: string, kind: "info" | "error" = "info") => {
    setStatusState({ msg, kind });
  }, []);

  /// Invalidates any in-flight request and clears the session. Called when the
  /// pop-in opens or closes.
  const reset = useCallback(() => {
    requests.current.cancel();
    setCandidates([]);
    setDetail(null);
    setStatusState({ msg: "", kind: "info" });
    setLoading({ on: false, label: "" });
    setSearching(false);
  }, []);

  const search = useCallback(
    async (rawQuery: string) => {
      const query = rawQuery.trim();
      if (!query) return;
      setCandidates([]);
      setSearching(true);
      setStatusState({ msg: "", kind: "info" });
      setLoading({ on: true, label: "Searching…" });

      // Each provider's failure is captured rather than thrown, so one being
      // down still lets the other's results through — but the reason is kept
      // so an *entirely* empty result can say why instead of a bare
      // "No results".
      const errors: string[] = [];
      const tasks: Promise<LookupCandidate[]>[] = [
        api.lookupMusicbrainz(query).catch((e) => {
          errors.push(`MusicBrainz: ${String(e)}`);
          return [];
        }),
      ];
      // Discogs requires the user's own token — silently skipped (not an
      // error) when none is set, same as leaving a provider unchecked.
      if (discogsToken) {
        tasks.push(
          api.lookupDiscogs(query, discogsToken).catch((e) => {
            errors.push(`Discogs: ${String(e)}`);
            return [];
          }),
        );
      }

      const result = await requests.current.run(() => Promise.all(tasks));
      if (result.status !== "success" || !result.isCurrent()) return;
      setLoading({ on: false, label: "" });
      setSearching(false);
      const found = result.value.flat();
      setCandidates(found);

      if (found.length > 0) {
        setStatusState({ msg: "", kind: "info" });
      } else if (errors.length > 0) {
        setStatusState({ msg: errors.join(" · "), kind: "error" });
      } else {
        setStatusState({ msg: "No results.", kind: "info" });
      }
      // Partial failure with results showing is a side note, not the headline.
      return found.length > 0 && errors.length > 0 ? errors.join(" · ") : undefined;
    },
    [discogsToken],
  );

  /// Loads a candidate's full track list + cover. Returns whether it actually
  /// loaded, so the "existing MusicBrainz ID" shortcut can fall back to a
  /// normal text search when the tagged ID turns out to be stale (release
  /// merged or deleted on MusicBrainz's side).
  const selectCandidate = useCallback(
    async (candidate: LookupCandidate): Promise<boolean> => {
      setStatusState({ msg: "", kind: "info" });
      setLoading({ on: true, label: "Loading track list…" });
      const result = await requests.current.run(() =>
        candidate.source === "MusicBrainz"
          ? api.lookupMusicbrainzDetail(candidate.id)
          : api.lookupDiscogsDetail(candidate.id, discogsToken),
      );
      if (result.status === "superseded" || !result.isCurrent()) return false;
      setLoading({ on: false, label: "" });
      if (result.status === "success") {
        setDetail(result.value);
        return true;
      }
      setStatusState({ msg: String(result.error), kind: "error" });
      return false;
    },
    [discogsToken],
  );

  /// Back to the candidate list, keeping the results already fetched.
  const clearDetail = useCallback(() => setDetail(null), []);

  return {
    candidates,
    detail,
    status,
    loading,
    searching,
    setStatus,
    reset,
    search,
    selectCandidate,
    clearDetail,
  };
}
