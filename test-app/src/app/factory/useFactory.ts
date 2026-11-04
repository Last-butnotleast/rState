import { useEffect, useState, useSyncExternalStore } from "react";
import { initializeRust } from "../../engines/rust";
import { FactorySession } from "./session";

const subscribeIdle = () => () => {};
const getIdle = () => undefined;

export function useFactory() {
  const [session, setSession] = useState<FactorySession>();
  const [error, setError] = useState<string>();
  useEffect(() => {
    let active = true;
    let current: FactorySession | undefined;
    initializeRust()
      .then(() => {
        if (!active) return;
        current = new FactorySession();
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
  }, []);
  const view = useSyncExternalStore(
    session?.subscribe ?? subscribeIdle,
    session?.getSnapshot ?? getIdle,
  );
  return { session, view, error };
}
