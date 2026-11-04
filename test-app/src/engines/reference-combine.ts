import { createAtom, type ReadonlyAtom } from "@reference/atom";
import { createStore, type SourceStore } from "@reference/store";
import { referenceConfig } from "./reference-config";
import type {
  CombineEngine,
  CombineMode,
  CombineStats,
  CombineTarget,
} from "./combine";
import type { SelectedValue } from "./selection";

type NumberValue = { value: number };

export function createReferenceCombine(mode: CombineMode): CombineEngine {
  const counts: CombineStats = {
    source: 0,
    left: 0,
    right: 0,
    combined: 0,
    chain: 0,
  };
  const leftStore = createStore(referenceConfig("counter", () => {}));
  const rightStore =
    mode === "independent"
      ? createStore(referenceConfig("counter", () => {}))
      : leftStore;
  const selection = (
    store: SourceStore,
    counter: "source" | "left" | "right",
  ) =>
    store.select((context) => {
      counts[counter]++;
      return { value: context.count };
    }, equal);
  let left: ReadonlyAtom<NumberValue>;
  let right: ReadonlyAtom<NumberValue>;
  if (mode === "independent") {
    left = selection(leftStore, "left");
    right = selection(rightStore, "right");
  } else {
    const source = selection(leftStore, "source");
    if (mode === "duplicate") {
      left = right = source;
    } else {
      left = createAtom(
        () => {
          counts.left++;
          return { value: source.get().value * 2 };
        },
        { compare: equal },
      );
      right = createAtom(
        () => {
          counts.right++;
          return { value: source.get().value + 1 };
        },
        { compare: equal },
      );
    }
  }
  const combined = createAtom(
    () => {
      counts.combined++;
      return { value: Math.floor((left.get().value + right.get().value) / 2) };
    },
    { compare: equal },
  );
  const chain = createAtom(
    () => {
      counts.chain++;
      return { value: `combined:${combined.get().value}` };
    },
    { compare: (previous, next) => previous.value === next.value },
  );
  const values = { left, right, combined, chain };
  const identities: Record<CombineTarget, WeakMap<object, SelectedValue>> = {
    left: new WeakMap(),
    right: new WeakMap(),
    combined: new WeakMap(),
    chain: new WeakMap(),
  };
  const next: Record<CombineTarget, number> = {
    left: 0,
    right: 0,
    combined: 0,
    chain: 0,
  };
  const retain = (
    target: CombineTarget,
    value: { value: number | string },
  ): SelectedValue => {
    const previous = identities[target].get(value);
    if (previous) return previous;
    const selected = { value: value.value, id: ++next[target] };
    identities[target].set(value, selected);
    return selected;
  };
  const subscriptions = new Set<() => void>();
  return {
    read: (target) => retain(target, values[target].get()),
    stats: () => ({ ...counts }),
    send: (input, event) =>
      (input === "left" ? leftStore : rightStore).send(event),
    subscribe: (target, callback) => {
      const subscription = values[target].subscribe((value) =>
        callback(retain(target, value)),
      );
      const unsubscribe = () => {
        subscription.unsubscribe();
        subscriptions.delete(unsubscribe);
      };
      subscriptions.add(unsubscribe);
      return unsubscribe;
    },
    dispose: () => {
      for (const unsubscribe of subscriptions) unsubscribe();
    },
  };
}

function equal(left: NumberValue, right: NumberValue): boolean {
  return left.value === right.value;
}
