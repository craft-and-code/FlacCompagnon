// Lookup pop-in, search step: the query box, Discogs credential settings, and
// the candidate list returned by the providers.

import type { LookupCandidate } from "../types";
import type { LookupStatus } from "./useLookup";
import type { DiscogsCredential } from "./useDiscogsCredential";
import { DiscogsCredentialSettings } from "./DiscogsCredentialSettings";
import { lookupCandidateKey } from "./lookupProviders";
import "./LookupSearchView.css";

export interface LookupSearchViewProps {
  query: string;
  onQueryChange: (query: string) => void;
  onSubmit: () => void;
  searching: boolean;
  discogsCredential: DiscogsCredential;
  status: LookupStatus;
  candidates: LookupCandidate[];
  onPick: (candidate: LookupCandidate) => void;
}

function candidateMeta(c: LookupCandidate): string {
  return [c.year, c.track_count ? `${c.track_count} tracks` : null].filter(Boolean).join(" · ");
}

export function LookupSearchView({
  query,
  onQueryChange,
  onSubmit,
  searching,
  discogsCredential,
  status,
  candidates,
  onPick,
}: LookupSearchViewProps) {
  return (
    <div className="lookup-view">
      <form
        className="lookup-search-row"
        onSubmit={(ev) => {
          ev.preventDefault();
          onSubmit();
        }}
      >
        <input
          type="text"
          placeholder="Artist – album"
          autoComplete="off"
          autoFocus
          value={query}
          onChange={(ev) => onQueryChange(ev.target.value)}
        />
        <button className="btn" type="submit" disabled={searching}>
          Search
        </button>
      </form>

      <DiscogsCredentialSettings credential={discogsCredential} />

      {status.msg && (
        <p
          className={
            status.kind === "error"
              ? "muted lookup-status lookup-status-error"
              : "muted lookup-status"
          }
        >
          {status.msg}
        </p>
      )}

      <div className="lookup-results">
        {candidates.map((c) => (
          <button
            type="button"
            className="lookup-candidate"
            key={lookupCandidateKey(c)}
            onClick={() => onPick(c)}
          >
            <span className="lookup-candidate-source">{c.source}</span>
            <span className="lookup-candidate-main">
              <span className="lookup-candidate-title">{c.title}</span>
              <span className="lookup-candidate-artist">{c.artist || "Unknown artist"}</span>
            </span>
            <span className="lookup-candidate-meta">{candidateMeta(c)}</span>
          </button>
        ))}
      </div>
    </div>
  );
}
