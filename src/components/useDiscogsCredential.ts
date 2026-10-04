// App owns one credential session even while the tag panel is closed. The
// backend exposes presence only; the OS credential value never enters React.

import { useCallback, useEffect, useState } from "react";
import * as api from "../api";
import {
  DiscogsCredentialController,
  initialDiscogsCredential,
  type DiscogsCredentialSnapshot,
} from "./discogsCredential";
import { useLatest } from "./useLatest";

export interface DiscogsCredential extends DiscogsCredentialSnapshot {
  readyForLookup: () => Promise<boolean>;
  save: (token: string) => Promise<boolean>;
  forget: () => Promise<boolean>;
  retry: () => Promise<boolean>;
}

export function useDiscogsCredential(onError: (message: string) => void): DiscogsCredential {
  const [snapshot, setSnapshot] = useState(initialDiscogsCredential);
  const notify = useLatest(onError);
  const [controller] = useState(
    () =>
      new DiscogsCredentialController(
        api,
        () => localStorage,
        setSnapshot,
        (message) => notify.current(message),
      ),
  );
  useEffect(() => {
    void controller.initialize();
  }, [controller]);
  const readyForLookup = useCallback(() => controller.readyForLookup(), [controller]);
  const save = useCallback((token: string) => controller.save(token), [controller]);
  const forget = useCallback(() => controller.forget(), [controller]);
  const retry = useCallback(() => controller.retry(), [controller]);
  return { ...snapshot, readyForLookup, save, forget, retry };
}
