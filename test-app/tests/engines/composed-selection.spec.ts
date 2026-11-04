import { expect, test } from "@playwright/test";

test("mapped chains are lazy and preserve cached value identity", async ({
  page,
}) => {
  await page.goto("/");
  await expect(
    page.getByRole("button", { name: "Increment left", exact: true }),
  ).toBeEnabled();
  const results = await page.evaluate(async () => {
    const rustPath = "/src/engines/rust-selection.ts";
    const referencePath = "/src/engines/reference-selection.ts";
    const { createRustSelection } = await import(rustPath);
    const { createReferenceSelection } = await import(referencePath);
    return [createRustSelection, createReferenceSelection].map((create) => {
      const engine = create();
      try {
        const initial = engine.stats();
        const first = engine.read("chain");
        const cached = engine.read("chain") === first;
        engine.send({ type: "inc", by: 1 });
        const beforeRead = engine.stats();
        const equal = engine.read("chain");
        const afterEqual = engine.stats();
        engine.send({ type: "inc", by: 1 });
        const changed = engine.read("chain");
        return {
          initial,
          first,
          cached,
          beforeRead,
          equal,
          equalIdentity: equal === first,
          afterEqual,
          changed,
          final: engine.stats(),
        };
      } finally {
        engine.dispose();
      }
    });
  });
  expect(results[0]).toEqual(results[1]);
  expect(results[0]).toMatchObject({
    initial: { source: 0, mapped: 0, chain: 0 },
    cached: true,
    beforeRead: { source: 1, mapped: 1, chain: 1 },
    equalIdentity: true,
    afterEqual: { source: 2, mapped: 2, chain: 1 },
    changed: { value: "bucket:1", id: 2 },
    final: { source: 3, mapped: 3, chain: 2 },
  });
});

test("mapped callbacks reconcile nested sends and drain errors like the reference", async ({
  page,
}) => {
  await page.goto("/");
  await expect(
    page.getByRole("button", { name: "Increment left", exact: true }),
  ).toBeEnabled();
  const results = await page.evaluate(async () => {
    const path = "/src/traces/selection.ts";
    const { compareSelectionScenario } = await import(path);
    return [
      {
        name: "reentry",
        steps: [
          {
            op: "subscribe",
            target: "mapped",
            listener: "a",
            whenValue: 1,
            sendBy: 2,
          },
          { op: "subscribe", target: "mapped", listener: "b" },
          { op: "subscribe", target: "chain", listener: "chain" },
          { op: "send", by: 2 },
          { op: "read", target: "chain" },
          { op: "stats" },
        ],
      },
      {
        name: "failure",
        steps: [
          {
            op: "subscribe",
            target: "mapped",
            listener: "bad",
            fail: "mapped listener failed",
          },
          { op: "subscribe", target: "mapped", listener: "good" },
          { op: "subscribe", target: "chain", listener: "chain" },
          { op: "send", by: 2 },
          { op: "unsubscribe", listener: "bad" },
          { op: "send", by: 2 },
          { op: "read", target: "chain" },
          { op: "stats" },
        ],
      },
    ].map((scenario) => compareSelectionScenario(scenario));
  });
  for (const result of results)
    expect(result.differences, JSON.stringify(result, null, 2)).toEqual([]);
  expect(
    results[0].rust.some(
      (entry: { kind: string }) => entry.kind === "nested-return",
    ),
  ).toBe(true);
  expect(
    results[1].rust.filter((entry: { kind: string }) => entry.kind === "error"),
  ).toHaveLength(1);
  expect(
    results[1].rust.filter(
      (entry: { listener?: string; kind: string }) =>
        entry.kind === "select" && entry.listener === "good",
    ),
  ).toHaveLength(2);
});

test("removing a direct source listener preserves a read-established chain cache", async ({
  page,
}) => {
  await page.goto("/");
  await expect(
    page.getByRole("button", { name: "Increment left", exact: true }),
  ).toBeEnabled();
  const results = await page.evaluate(async () => {
    const rustPath = "/src/engines/rust-selection.ts";
    const referencePath = "/src/engines/reference-selection.ts";
    const { createRustSelection } = await import(rustPath);
    const { createReferenceSelection } = await import(referencePath);
    return [createRustSelection, createReferenceSelection].map((create) => {
      const engine = create();
      try {
        const first = engine.read("chain");
        const unsubscribe = engine.subscribe("source", () => {});
        unsubscribe();
        const afterDetach = engine.read("chain");
        const detachedCounts = engine.stats();
        engine.send({ type: "inc", by: 2 });
        const unobservedCounts = engine.stats();
        const changed = engine.read("chain");
        return {
          retainedIdentity: first === afterDetach,
          detachedCounts,
          unobservedCounts,
          changed,
          finalCounts: engine.stats(),
        };
      } finally {
        engine.dispose();
      }
    });
  });
  expect(results[0]).toEqual(results[1]);
  expect(results[0]).toEqual({
    retainedIdentity: true,
    detachedCounts: { source: 1, mapped: 1, chain: 1 },
    unobservedCounts: { source: 1, mapped: 1, chain: 1 },
    changed: { value: "bucket:1", id: 2 },
    finalCounts: { source: 2, mapped: 2, chain: 2 },
  });
});
