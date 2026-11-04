import { useEffect, useState, useSyncExternalStore } from "react";
import type { Scenario } from "../engines/contract";
import { initializeRust } from "../engines/rust";
import { LabSession } from "./session";

const subscribeIdle = () => () => {};
const getIdle = () => undefined;

export function useLab(scenario: Scenario) {
  const [session, setSession] = useState<LabSession>();
  const [error, setError] = useState<string>();
  useEffect(() => {
    let active = true;
    let current: LabSession | undefined;
    setSession(undefined);
    initializeRust()
      .then(() => {
        if (!active) return;
        current = new LabSession(scenario);
        setSession(current);
      })
      .catch((failure: unknown) => {
        if (active)
          setError(
            failure instanceof Error ? failure.message : String(failure),
          );
      });
    return () => {
      active = false;
      current?.dispose();
    };
  }, [scenario]);
  const view = useSyncExternalStore(
    session?.subscribe ?? subscribeIdle,
    session?.getSnapshot ?? getIdle,
  );
  return { session, view, error };
}
