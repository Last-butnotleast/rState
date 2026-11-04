import { useEffect, useState, useSyncExternalStore } from "react";
import { initializeRust } from "../../engines/rust";
import { SelectionSession } from "./session";

const subscribeIdle = () => () => {};
const getIdle = () => undefined;

export function useSelection() {
  const [session, setSession] = useState<SelectionSession>();
  const [error, setError] = useState<string>();
  useEffect(() => {
    let active = true;
    let current: SelectionSession | undefined;
    initializeRust()
      .then(() => {
        if (!active) return;
        current = new SelectionSession();
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
