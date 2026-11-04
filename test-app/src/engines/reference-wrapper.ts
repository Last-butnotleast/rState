import type { SourceStore } from "@reference/store";
import type { Context, Engine, Snapshot } from "./contract";

export function wrapReference(store: SourceStore): Engine {
  const subscriptions: (() => void)[] = [];
  const snapshots = new WeakMap<object, number>();
  const contexts = new WeakMap<object, number>();
  let nextSnapshot = 0;
  let nextContext = 0;
  const snapshot = (value: { context: Context }): Snapshot => {
    if (!snapshots.has(value)) snapshots.set(value, ++nextSnapshot);
    if (!contexts.has(value.context))
      contexts.set(value.context, ++nextContext);
    return {
      context: value.context,
      snapshotId: snapshots.get(value)!,
      contextId: contexts.get(value.context)!,
    };
  };
  const register = (unsubscribe: () => void) => {
    subscriptions.push(unsubscribe);
    return unsubscribe;
  };
  return {
    snapshot: () => snapshot(store.getSnapshot()),
    initialSnapshot: () => snapshot(store.getInitialSnapshot()),
    send: (event) => store.send(event),
    can: (event) => store.can[event.type]?.(event) ?? false,
    transition: (event) => {
      const current = store.getSnapshot();
      const result = store.transition(current, event);
      return {
        snapshot: snapshot(result[0]),
        effects: result[1].length,
        allowed:
          result._allowed ?? (result[0] !== current || result[1].length > 0),
      };
    },
    subscribe: (callback) => {
      const subscription = store.subscribe((value) =>
        callback(snapshot(value)),
      );
      return register(() => subscription.unsubscribe());
    },
    subscribeSelected: (field, callback) => {
      const selection = store.select((context) => context[field], Object.is);
      const subscription = selection.subscribe(callback);
      return register(() => subscription.unsubscribe());
    },
    onReceipt: (callback) => {
      const subscription = store.on("receipt", callback);
      return register(() => subscription.unsubscribe());
    },
    onAny: (callback) => {
      const subscription = store.on("*", callback);
      return register(() => subscription.unsubscribe());
    },
    dispose: () =>
      subscriptions.splice(0).forEach((unsubscribe) => unsubscribe()),
  };
}
