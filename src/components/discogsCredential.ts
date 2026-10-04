// Startup removes the legacy browser credential before migrating it. A failed
// migration retains only a session copy for retry; snapshots never expose it.

import type { DiscogsCredentialStatus } from "../types";

export const LEGACY_DISCOGS_TOKEN_KEY = "flaccompagnon.discogsToken";

export interface DiscogsCredentialSnapshot {
  configured: boolean;
  ready: boolean;
  busy: boolean;
  error: string | null;
  recoveryPending: boolean;
  legacyCleanupPending: boolean;
}

export interface DiscogsCredentialBackend {
  getDiscogsCredentialStatus(): Promise<DiscogsCredentialStatus>;
  saveDiscogsToken(token: string): Promise<DiscogsCredentialStatus>;
  deleteDiscogsToken(): Promise<DiscogsCredentialStatus>;
  migrateDiscogsToken(token: string): Promise<DiscogsCredentialStatus>;
}

type LegacyStorage = Pick<Storage, "getItem" | "removeItem">;

export function initialDiscogsCredential(): DiscogsCredentialSnapshot {
  return {
    configured: false,
    ready: false,
    busy: false,
    error: null,
    recoveryPending: false,
    legacyCleanupPending: false,
  };
}

function vaultError(action: string): string {
  // Native error text can include request details. Keep secrets out of UI
  // errors while still explaining the action and how to retry a locked store.
  return `Could not ${action} the Discogs token in system credential storage. Unlock the store or make it available, then retry.`;
}

export class DiscogsCredentialController {
  private state = initialDiscogsCredential();
  private recoveryToken: string | null = null;
  private initialization: Promise<boolean> | null = null;
  private mutation: Promise<boolean> | null = null;
  private forgetting = false;

  constructor(
    private backend: DiscogsCredentialBackend,
    private storage: () => LegacyStorage,
    private publish: (state: DiscogsCredentialSnapshot) => void,
    private onError: (message: string) => void,
  ) {}

  initialize(): Promise<boolean> {
    // StrictMode replays startup effects. Both callers must share the same
    // removal/migration rather than racing two writes against the vault.
    this.initialization ??= this.reconcile();
    return this.initialization;
  }

  async readyForLookup(): Promise<boolean> {
    await this.initialize();
    if (this.mutation) await this.mutation;
    return this.state.configured;
  }

  save(token: string): Promise<boolean> {
    const trimmed = token.trim();
    if (!trimmed) return Promise.resolve(false);
    return this.change(async () => {
      const errors: string[] = [];
      this.start();
      this.removeLegacy(errors);
      let configured = this.state.configured;
      let saved = false;
      try {
        configured = (await this.backend.saveDiscogsToken(trimmed)).configured;
        saved = configured;
        if (saved) {
          this.recoveryToken = null;
          this.forgetting = false;
        } else errors.push(vaultError("save"));
      } catch {
        errors.push(vaultError("save"));
      }
      this.finish(configured, errors);
      return saved;
    });
  }

  forget(): Promise<boolean> {
    return this.change(() => this.forgetToken());
  }

  retry(): Promise<boolean> {
    return this.change(() => (this.forgetting ? this.forgetToken() : this.reconcile()));
  }

  private async change(action: () => Promise<boolean>): Promise<boolean> {
    await this.initialize();
    // Credential writes are exclusive: an older save/delete response must
    // never describe a newer operation that is still pending.
    if (this.mutation) return false;
    const pending = action();
    this.mutation = pending;
    try {
      return await pending;
    } finally {
      if (this.mutation === pending) this.mutation = null;
    }
  }

  private async reconcile(): Promise<boolean> {
    const errors: string[] = [];
    this.start();
    this.readLegacy(errors);
    let configured = false;
    try {
      const status = this.recoveryToken
        ? await this.backend.migrateDiscogsToken(this.recoveryToken)
        : await this.backend.getDiscogsCredentialStatus();
      configured = status.configured;
      if (configured) this.recoveryToken = null;
      else if (this.recoveryToken) errors.push(vaultError("migrate"));
    } catch {
      errors.push(vaultError(this.recoveryToken ? "migrate" : "check"));
    }
    if (this.recoveryToken) {
      errors.push(
        "A recovery copy is held in memory for this app session for Retry or Forget. Retry before closing the app to avoid losing it.",
      );
    }
    this.finish(configured, errors);
    return configured;
  }

  private async forgetToken(): Promise<boolean> {
    const errors: string[] = [];
    this.start();
    this.forgetting = true;
    this.recoveryToken = null;
    this.removeLegacy(errors);
    // Deleting the vault entry while browser cleanup fails would let startup
    // migrate the surviving plaintext back into the vault after a reload.
    if (this.state.legacyCleanupPending) {
      this.finish(this.state.configured, errors);
      return false;
    }
    let configured = this.state.configured;
    let forgotten = false;
    try {
      configured = (await this.backend.deleteDiscogsToken()).configured;
      forgotten = !configured;
      if (forgotten) this.forgetting = false;
      else errors.push(vaultError("remove"));
    } catch {
      errors.push(vaultError("remove"));
    }
    this.finish(configured, errors);
    return forgotten;
  }

  private readLegacy(errors: string[]): void {
    try {
      const storage = this.storage();
      const legacy = storage.getItem(LEGACY_DISCOGS_TOKEN_KEY);
      if (legacy == null) {
        this.state = { ...this.state, legacyCleanupPending: false };
        return;
      }
      this.recoveryToken ??= legacy.trim() || null;
      this.removeLegacy(errors);
    } catch {
      this.state = { ...this.state, legacyCleanupPending: true };
      errors.push(
        "Could not inspect the old browser token storage. Its plaintext copy may still remain; retry cleanup when storage is available.",
      );
    }
  }

  private removeLegacy(errors: string[]): void {
    try {
      this.storage().removeItem(LEGACY_DISCOGS_TOKEN_KEY);
      this.state = { ...this.state, legacyCleanupPending: false };
    } catch {
      this.state = { ...this.state, legacyCleanupPending: true };
      errors.push(
        "Could not remove the old browser token. Its plaintext copy may still remain; retry cleanup when storage is available.",
      );
    }
  }

  private start(): void {
    this.state = { ...this.state, busy: true, error: null };
    this.publish(this.state);
  }

  private finish(configured: boolean, errors: string[]): void {
    this.state = {
      ...this.state,
      configured,
      ready: true,
      busy: false,
      recoveryPending: this.recoveryToken != null,
      error: errors.length ? errors.join(" ") : null,
    };
    this.publish(this.state);
    if (this.state.error) this.onError(this.state.error);
  }
}
