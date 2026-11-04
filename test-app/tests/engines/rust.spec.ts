import { expect, test } from "@playwright/test";

test("real Rust store returns stable identities, synchronous callbacks and explicit unsubscribe", async ({
  page,
}) => {
  await page.goto("/");
  await expect(
    page.getByRole("button", { name: "Increment", exact: true }),
  ).toBeEnabled();
  const result = await page.evaluate(async () => {
    const path = "/src/engines/rust.ts";
    const { createRust } = await import(path);
    const engine = createRust("counter", () => {});
    const before = engine.snapshot();
    const trace: string[] = [];
    const unsubscribe = engine.subscribe(() => trace.push("callback"));
    engine.send({ type: "inc", by: 1 });
    trace.push("returned");
    const after = engine.snapshot();
    const repeated = engine.snapshot();
    unsubscribe();
    unsubscribe();
    engine.send({ type: "inc", by: 1 });
    const count = engine.snapshot().context.count;
    engine.dispose();
    return {
      trace,
      changed: before !== after,
      stable: repeated === after,
      count,
    };
  });
  expect(result).toEqual({
    trace: ["callback", "returned"],
    changed: true,
    stable: true,
    count: 2,
  });
});

test("Rust fixture overflow preserves committed state and remains usable", async ({
  page,
}) => {
  await page.goto("/");
  await expect(
    page.getByRole("button", { name: "Increment", exact: true }),
  ).toBeEnabled();
  const results = await page.evaluate(async () => {
    const path = "/src/generated/rstate_wasm.js";
    const { default: initialize, FixtureStore } = await import(path);
    await initialize();
    return ["counter", "composition"].map((scenario) => {
      const store = new FixtureStore(scenario, () => {});
      const notifications: string[] = [];
      const emissions: string[] = [];
      store.subscribe((snapshot: string) => notifications.push(snapshot));
      store.on_any((message: string) => emissions.push(message));
      const maximum =
        scenario === "counter"
          ? '{"type":"inc","by":9223372036854775807}'
          : '{"type":"add","price":9223372036854775807}';
      const overflow =
        scenario === "counter"
          ? '{"type":"inc","by":1}'
          : '{"type":"add","price":1}';
      try {
        store.send(maximum);
        const before = store.snapshot();
        const errors = [
          () => store.can(overflow),
          () => store.transition(overflow),
          () => store.send(overflow),
        ].map((run) => {
          try {
            run();
            return "no error";
          } catch (failure) {
            return failure instanceof Error ? failure.message : String(failure);
          }
        });
        const unchanged = before === store.snapshot();
        const afterFailure = {
          notifications: notifications.length,
          emissions: emissions.length,
        };
        store.send(
          scenario === "counter"
            ? '{"type":"inc","by":-9223372036854775807}'
            : '{"type":"add","price":-9223372036854775807}',
        );
        store.send(overflow);
        const context = JSON.parse(store.snapshot()).context;
        return {
          scenario,
          errors,
          unchanged,
          afterFailure,
          recoveredValue:
            scenario === "counter" ? context.count : context.total,
          finalNotifications: notifications.length,
        };
      } finally {
        store.dispose();
        store.free();
      }
    });
  });
  expect(results).toEqual([
    {
      scenario: "counter",
      errors: ["count overflow", "count overflow", "count overflow"],
      unchanged: true,
      afterFailure: { notifications: 1, emissions: 0 },
      recoveredValue: 1,
      finalNotifications: 3,
    },
    {
      scenario: "composition",
      errors: ["total overflow", "total overflow", "total overflow"],
      unchanged: true,
      afterFailure: { notifications: 1, emissions: 1 },
      recoveredValue: 1,
      finalNotifications: 3,
    },
  ]);
});
