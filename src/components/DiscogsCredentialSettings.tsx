// The form contains only a newly entered draft. A saved OS credential is
// represented by its presence indicator and is never read back into the input.

import { useEffect, useRef, useState } from "react";
import type { DiscogsCredential } from "./useDiscogsCredential";
import { DraftRevision } from "./draftRevision";
import { finishCredentialDraft } from "./credentialDraft";
import "./DiscogsCredentialSettings.css";

export interface DiscogsCredentialSettingsProps {
  credential: DiscogsCredential;
}

export function DiscogsCredentialSettings({ credential }: DiscogsCredentialSettingsProps) {
  const [draft, setDraft] = useState("");
  const [expanded, setExpanded] = useState(false);
  const revision = useRef(new DraftRevision("discogs"));
  useEffect(() => {
    if (credential.error || credential.recoveryPending || credential.legacyCleanupPending) {
      setExpanded(true);
    }
  }, [credential.error, credential.recoveryPending, credential.legacyCleanupPending]);

  const unavailable = !credential.ready || credential.busy;
  const forgettable =
    credential.configured || credential.recoveryPending || credential.legacyCleanupPending;
  const status = !credential.ready
    ? "Checking system credential storage…"
    : credential.busy
      ? "Updating system credential storage…"
      : credential.configured
        ? "Token saved in system credential storage."
        : credential.error
          ? "Discogs is unavailable until secure storage is ready."
          : "No Discogs token saved.";

  return (
    <details
      className="discogs-settings"
      open={expanded}
      onToggle={(event) => setExpanded(event.currentTarget.open)}
    >
      <summary>Discogs token{credential.configured ? " — saved" : ""}</summary>
      <p className="muted">
        Optional. MusicBrainz remains available without a saved Discogs token. Get a personal access
        token from discogs.com → Settings → Developers.
      </p>
      <p className="discogs-settings-status" role="status">
        {status}
      </p>
      <input
        type="password"
        placeholder={
          credential.configured ? "Enter a replacement token" : "Discogs personal access token"
        }
        aria-label="New Discogs personal access token"
        autoComplete="off"
        spellCheck={false}
        value={draft}
        onChange={(event) => {
          revision.current.touch();
          setDraft(event.target.value);
        }}
      />
      <div className="discogs-settings-actions">
        <button
          className="btn btn-secondary"
          type="button"
          disabled={unavailable || !draft.trim()}
          onClick={() =>
            void finishCredentialDraft(
              revision.current,
              () => credential.save(draft),
              () => setDraft(""),
            )
          }
        >
          Save token
        </button>
        <button
          className="btn btn-ghost"
          type="button"
          disabled={unavailable || !forgettable}
          onClick={() =>
            void finishCredentialDraft(revision.current, credential.forget, () => setDraft(""))
          }
        >
          Forget token
        </button>
        {(credential.error || credential.recoveryPending || credential.legacyCleanupPending) && (
          <button
            className="btn btn-ghost"
            type="button"
            disabled={unavailable}
            onClick={() => void credential.retry()}
          >
            Retry secure storage
          </button>
        )}
      </div>
      {credential.recoveryPending && (
        <p className="muted">
          A recovery copy is held in memory for this session. Retry or Forget before closing the
          app.
        </p>
      )}
      {credential.error && (
        <p className="discogs-settings-error" role="alert">
          {credential.error}
        </p>
      )}
    </details>
  );
}
