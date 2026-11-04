import { createAtom } from "@reference/atom";
import { createStore } from "@reference/store";
import { referenceConfig } from "./reference-config";
import type {
  SelectedValue,
  SelectionEngine,
  SelectionStats,
  SelectionTarget,
} from "./selection";

export function createReferenceSelection(): SelectionEngine {
  const store = createStore(referenceConfig("counter", () => {}));
  const counts: SelectionStats = { source: 0, mapped: 0, chain: 0 };
  const source = store.select(
    (context) => {
      counts.source++;
      return { value: context.count };
    },
    (left, right) => left.value === right.value,
  );
  const mapped = createAtom(
    () => {
      counts.mapped++;
      return { value: Math.floor(source.get().value / 2) };
    },
    { compare: (left, right) => left.value === right.value },
  );
  const chain = createAtom(
    () => {
      counts.chain++;
      return { value: `bucket:${mapped.get().value}` };
    },
    { compare: (left, right) => left.value === right.value },
  );
  const selections = { source, mapped, chain };
  const identities: Record<SelectionTarget, WeakMap<object, SelectedValue>> = {
    source: new WeakMap(),
    mapped: new WeakMap(),
    chain: new WeakMap(),
  };
  const next: SelectionStats = { source: 0, mapped: 0, chain: 0 };
  const subscriptions = new Set<() => void>();
  const serialize = (
    target: SelectionTarget,
    value: { value: number | string },
  ): SelectedValue => {
    const previous = identities[target].get(value);
    if (previous) return previous;
    const result = { value: value.value, id: ++next[target] };
    identities[target].set(value, result);
    return result;
  };
  return {
    read: (target) => serialize(target, selections[target].get()),
    stats: () => ({ ...counts }),
    send: (event) => store.send(event),
    subscribe: (target, callback) => {
      const subscription = selections[target].subscribe((value) =>
        callback(serialize(target, value)),
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
