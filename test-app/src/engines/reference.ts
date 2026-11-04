import { createStore } from "@reference/store";
import type {
  Context,
  Engine,
  Event,
  Recorder,
  ScenarioName,
  Snapshot,
} from "./contract";

export function createReference(
  scenario: ScenarioName,
  record: Recorder,
): Engine {
  const subscriptions: (() => void)[] = [];
  const snapshots = new WeakMap<object, number>();
  const contexts = new WeakMap<object, number>();
  let nextSnapshot = 0;
  let nextContext = 0;
  const store = createStore({
    context: { count: 0, items: [], total: 0 },
    on: {
      inc: (context, event) => ({
        ...context,
        count: context.count + (event.by ?? 0),
      }),
      bump: (context, event) => ({
        ...context,
        count: context.count + (event.by ?? 0),
      }),
      keep: () => undefined,
      same: (context) => context,
      equal: (context) => ({ ...context }),
      fail: () => {
        throw new Error("reducer failed");
      },
      add: (context, event, enqueue) => {
        if (scenario !== "composition") return;
        enqueue.trigger.recalculate();
        return { ...context, items: [...context.items, event.price ?? 0] };
      },
      recalculate: (context, _event, enqueue) => {
        if (scenario !== "composition") return;
        enqueue.trigger.receipt();
        return {
          ...context,
          total: context.items.reduce((sum, price) => sum + price, 0),
        };
      },
      receipt: (context, _event, enqueue) => {
        if (scenario !== "composition") return;
        enqueue.emit.receipt({ total: context.total });
      },
      start: (context, _event, enqueue) => {
        if (scenario !== "ordering") return;
        enqueue.effect((scope) => {
          record({
            kind: "effect",
            label: "before",
            count: scope.getSnapshot().context.count,
          });
          scope.send({ type: "bump", by: 10 });
        });
        enqueue.effect((scope) => {
          record({
            kind: "effect",
            label: "after",
            count: scope.getSnapshot().context.count,
          });
        });
        return { ...context, count: context.count + 1 };
      },
      effectFail: (context, _event, enqueue) => {
        if (scenario !== "ordering") return;
        enqueue.effect(() => {
          throw new Error("effect failed");
        });
        return { ...context, count: context.count + 1 };
      },
    },
  });
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
