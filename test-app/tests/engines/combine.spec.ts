import { expect, test } from "@playwright/test";

test("two-source selections stay lazy and retain equal combined identities", async ({
  page,
}) => {
  await page.goto("/");
  await expect(
    page.getByRole("button", { name: "Increment left", exact: true }),
  ).toBeEnabled();
  const results = await page.evaluate(async () => {
    const rustPath = "/src/engines/rust-combine.ts";
    const referencePath = "/src/engines/reference-combine.ts";
    const { createRustCombine } = await import(rustPath);
    const { createReferenceCombine } = await import(referencePath);
    return ["independent", "diamond", "duplicate"].map((mode) => ({
      mode,
      outcomes: [createRustCombine, createReferenceCombine].map((create) => {
        const engine = create(mode);
        try {
          const initial = engine.stats();
          const first = engine.read("combined");
          const cached = first === engine.read("combined");
          const beforeSend = engine.stats();
          engine.send("left", { type: "inc", by: 1 });
          const beforeRead = engine.stats();
          const next = engine.read("combined");
          return {
            initial,
            first,
            cached,
            beforeSend,
            beforeRead,
            next,
            retained: first === next,
            final: engine.stats(),
          };
        } finally {
          engine.dispose();
        }
      }),
    }));
  });
  for (const { outcomes } of results) {
    expect(outcomes[0]).toEqual(outcomes[1]);
    expect(outcomes[0].initial).toEqual({
      source: 0,
      left: 0,
      right: 0,
      combined: 0,
      chain: 0,
    });
    expect(outcomes[0].cached).toBe(true);
    expect(outcomes[0].beforeRead).toEqual(outcomes[0].beforeSend);
  }
  expect(results.map(({ outcomes }) => outcomes[0].next.value)).toEqual([
    0, 2, 1,
  ]);
  expect(results.map(({ outcomes }) => outcomes[0].retained)).toEqual([
    true,
    false,
    false,
  ]);
});

test("diamonds and duplicate inputs publish one coherent combined result", async ({
  page,
}) => {
  await page.goto("/");
  await expect(
    page.getByRole("button", { name: "Increment left", exact: true }),
  ).toBeEnabled();
  const results = await page.evaluate(async () => {
    const rustPath = "/src/engines/rust-combine.ts";
    const referencePath = "/src/engines/reference-combine.ts";
    const { createRustCombine } = await import(rustPath);
    const { createReferenceCombine } = await import(referencePath);
    return ["diamond", "duplicate"].map((mode) =>
      [createRustCombine, createReferenceCombine].map((create) => {
        const engine = create(mode);
        const values: number[] = [];
        try {
          engine.subscribe("combined", (value: { value: number }) =>
            values.push(value.value),
          );
          const before = [...values];
          engine.send("left", { type: "inc", by: 1 });
          return {
            before,
            values,
            current: engine.read("combined"),
            counts: engine.stats(),
          };
        } finally {
          engine.dispose();
        }
      }),
    );
  });
  expect(results[0][0]).toEqual(results[0][1]);
  expect(results[1][0]).toEqual(results[1][1]);
  expect(results[0][0].before).toEqual([]);
  expect(results[0][0].values).toEqual([2]);
  expect(results[1][0].values).toEqual([1]);
  expect(results[0][0].counts).toEqual({
    source: 2,
    left: 2,
    right: 2,
    combined: 2,
    chain: 0,
  });
  expect(results[1][0].counts).toEqual({
    source: 2,
    left: 0,
    right: 0,
    combined: 2,
    chain: 0,
  });
});

test("independent stores keep separate commits and combined observation detaches", async ({
  page,
}) => {
  await page.goto("/");
  await expect(
    page.getByRole("button", { name: "Increment left", exact: true }),
  ).toBeEnabled();
  const results = await page.evaluate(async () => {
    const rustPath = "/src/engines/rust-combine.ts";
    const referencePath = "/src/engines/reference-combine.ts";
    const { createRustCombine } = await import(rustPath);
    const { createReferenceCombine } = await import(referencePath);
    return [createRustCombine, createReferenceCombine].map((create) => {
      const engine = create("independent");
      const values: number[] = [];
      try {
        const unsubscribe = engine.subscribe(
          "combined",
          (value: { value: number }) => values.push(value.value),
        );
        engine.send("left", { type: "inc", by: 2 });
        engine.send("right", { type: "inc", by: 2 });
        unsubscribe();
        const detached = engine.stats();
        engine.send("left", { type: "inc", by: 2 });
        const beforeRead = engine.stats();
        const current = engine.read("combined");
        const again = engine.subscribe("combined", (value: { value: number }) =>
          values.push(value.value),
        );
        const beforeSend = [...values];
        engine.send("right", { type: "inc", by: 2 });
        again();
        return {
          values,
          detached,
          beforeRead,
          current,
          beforeSend,
          final: engine.stats(),
        };
      } finally {
        engine.dispose();
      }
    });
  });
  expect(results[0]).toEqual(results[1]);
  expect(results[0].values).toEqual([1, 2, 4]);
  expect(results[0].beforeSend).toEqual([1, 2]);
  expect(results[0].beforeRead).toEqual(results[0].detached);
  expect(results[0].current.value).toBe(3);
});

test("combined callbacks preserve nested dispatch and drain failures", async ({
  page,
}) => {
  await page.goto("/");
  await expect(
    page.getByRole("button", { name: "Increment left", exact: true }),
  ).toBeEnabled();
  const results = await page.evaluate(async () => {
    const tracePath = "/src/traces/combine.ts";
    const { compareCombineScenario } = await import(tracePath);
    return [
      {
        name: "cross-store nested send",
        mode: "independent",
        steps: [
          {
            op: "subscribe",
            target: "combined",
            listener: "a",
            whenValue: 1,
            sendInput: "right",
            sendBy: 2,
          },
          { op: "subscribe", target: "combined", listener: "b" },
          { op: "subscribe", target: "chain", listener: "chain" },
          { op: "send", input: "left", by: 2 },
          { op: "read", target: "combined" },
          { op: "stats" },
        ],
      },
      {
        name: "diamond callback failure",
        mode: "diamond",
        steps: [
          {
            op: "subscribe",
            target: "combined",
            listener: "bad",
            fail: "combined listener failed",
          },
          { op: "subscribe", target: "combined", listener: "good" },
          { op: "subscribe", target: "chain", listener: "chain" },
          { op: "send", input: "left", by: 1 },
          { op: "unsubscribe", listener: "bad" },
          { op: "send", input: "left", by: 1 },
          { op: "read", target: "chain" },
          { op: "stats" },
        ],
      },
    ].map((scenario) => compareCombineScenario(scenario));
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
      (entry: { kind: string; listener?: string }) =>
        entry.kind === "select" && entry.listener === "good",
    ),
  ).toHaveLength(2);
});
