import type { Handler } from "@reference/store";
import type { Context, Recorder, ScenarioName } from "./contract";

export function referenceConfig(
  scenario: ScenarioName,
  record: Recorder,
): { context: Context; on: Record<string, Handler> } {
  return {
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
  };
}
