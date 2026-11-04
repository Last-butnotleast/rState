import { FixtureCombined } from "../generated/rstate_wasm";
import type { CombineEngine, CombineMode, CombineTarget } from "./combine";
import type { SelectedValue } from "./selection";

export function createRustCombine(mode: CombineMode): CombineEngine {
  const fixture = new FixtureCombined(mode);
  const identities: Record<CombineTarget, Map<number, SelectedValue>> = {
    left: new Map(),
    right: new Map(),
    combined: new Map(),
    chain: new Map(),
  };
  const decode = (target: CombineTarget, serialized: string) => {
    const value = JSON.parse(serialized) as SelectedValue;
    const previous = identities[target].get(value.id);
    if (previous) return previous;
    identities[target].set(value.id, value);
    return value;
  };
  let disposed = false;
  return {
    read: (target) => decode(target, fixture.read(target)),
    stats: () => JSON.parse(fixture.stats()),
    send: (input, event) => fixture.send(input, JSON.stringify(event)),
    subscribe: (target, callback) => {
      const id = fixture.subscribe(target, (serialized: string) =>
        callback(decode(target, serialized)),
      );
      return () => {
        if (!disposed) fixture.unsubscribe(id);
      };
    },
    dispose: () => {
      if (disposed) return;
      fixture.dispose();
      fixture.free();
      disposed = true;
    },
  };
}
