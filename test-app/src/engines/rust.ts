import init, { FixtureStore } from "../generated/rstate_wasm";
import type {
  Engine,
  Inspection,
  Recorder,
  ScenarioName,
  Snapshot,
} from "./contract";

let initialized: Promise<unknown> | undefined;

export async function initializeRust() {
  await (initialized ??= init());
}

export function createRust(scenario: ScenarioName, record: Recorder): Engine {
  const store = new FixtureStore(scenario, (entry: string) =>
    record(JSON.parse(entry)),
  );
  return wrapRustStore(store);
}

export function wrapRustStore(store: FixtureStore): Engine {
  const snapshots = new Map<number, Snapshot>();
  const retain = (next: Snapshot): Snapshot => {
    const previous = snapshots.get(next.snapshotId);
    if (previous) return previous;
    snapshots.set(next.snapshotId, next);
    return next;
  };
  const decode = (serialized: string): Snapshot =>
    retain(JSON.parse(serialized));
  let disposed = false;
  return {
    snapshot: () => decode(store.snapshot()),
    initialSnapshot: () => decode(store.initial_snapshot()),
    send: (event) => store.send(JSON.stringify(event)),
    can: (event) => store.can(JSON.stringify(event)),
    transition: (event) => JSON.parse(store.transition(JSON.stringify(event))),
    subscribe: (callback) => {
      const id = store.subscribe((serialized: string) =>
        callback(decode(serialized)),
      );
      return () => {
        if (!disposed) store.unsubscribe(id);
      };
    },
    inspect: (callback) => {
      const id = store.inspect((serialized: string) => {
        const record = JSON.parse(serialized) as Inspection;
        callback({
          event: record.event,
          snapshot: retain(record.snapshot),
        });
      });
      return () => {
        if (!disposed) store.unsubscribe(id);
      };
    },
    subscribeSelected: (field, callback) => {
      const id = store.subscribe_selected(field, (serialized: string) =>
        callback(JSON.parse(serialized)),
      );
      return () => {
        if (!disposed) store.unsubscribe(id);
      };
    },
    onReceipt: (callback) => {
      const id = store.on_receipt((serialized: string) =>
        callback(JSON.parse(serialized)),
      );
      return () => {
        if (!disposed) store.unsubscribe(id);
      };
    },
    onAny: (callback) => {
      const id = store.on_any((serialized: string) =>
        callback(JSON.parse(serialized)),
      );
      return () => {
        if (!disposed) store.unsubscribe(id);
      };
    },
    dispose: () => {
      if (disposed) return;
      store.dispose();
      store.free();
      disposed = true;
    },
  };
}
