// Lookup pop-in, search step: the query box, the optional Discogs token, and
// the candidate list returned by the providers.

import type { LookupCandidate } from "../types";
import type { LookupStatus } from "./useLookup";

export interface LookupSearchViewProps {
  query: string;
  onQueryChange: (query: string) => void;
  onSubmit: () => void;
  searching: boolean;
  discogsToken: string;
  onDiscogsTokenChange: (token: string) => void;
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
  discogsToken,
  onDiscogsTokenChange,
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

      <details className="lookup-settings">
        <summary>Discogs token</summary>
        <p className="muted">
          Optional — without it, only MusicBrainz results are searched. Get a personal access token
          from Discogs' developer settings (discogs.com → Settings → Developers).
        </p>
        <input
          type="text"
          placeholder="Discogs personal access token"
          autoComplete="off"
          value={discogsToken}
          onChange={(ev) => onDiscogsTokenChange(ev.target.value)}
        />
      </details>

      {status.msg && (
        <p className={status.kind === "error" ? "muted lookup-status lookup-status-error" : "muted lookup-status"}>
          {status.msg}
        </p>
      )}

      <div className="lookup-results">
        {candidates.map((c, i) => (
          <button
            type="button"
            className="lookup-candidate"
            key={`${c.source}:${c.id}:${i}`}
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
