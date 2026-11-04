import { FixtureSelection } from "../generated/rstate_wasm";
import type {
  SelectedValue,
  SelectionEngine,
  SelectionTarget,
} from "./selection";

export function createRustSelection(): SelectionEngine {
  const fixture = new FixtureSelection();
  const values: Record<SelectionTarget, Map<number, SelectedValue>> = {
    source: new Map(),
    mapped: new Map(),
    chain: new Map(),
  };
  let disposed = false;
  const decode = (target: SelectionTarget, serialized: string) => {
    const next = JSON.parse(serialized) as SelectedValue;
    const previous = values[target].get(next.id);
    if (previous) return previous;
    values[target].set(next.id, next);
    return next;
  };
  return {
    read: (target) => decode(target, fixture.read(target)),
    stats: () => JSON.parse(fixture.stats()),
    send: (event) => fixture.send(JSON.stringify(event)),
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
