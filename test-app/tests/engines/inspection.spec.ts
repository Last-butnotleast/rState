import { expect, test } from "@playwright/test";

test("inspection preserves typed fixture event contents across every scenario", async ({
  page,
}) => {
  await page.goto("/");
  await expect(
    page.getByRole("button", { name: "Increment", exact: true }),
  ).toBeEnabled();
  const failures = await page.evaluate(async () => {
    const path = "/src/traces/compare.ts";
    const { compareScenario } = await import(path);
    const events = [
      { type: "inc", by: 2 },
      { type: "bump", by: 2 },
      { type: "keep" },
      { type: "same" },
      { type: "equal" },
      { type: "add", price: 8 },
      { type: "recalculate" },
      { type: "receipt" },
      { type: "start" },
      { type: "fail" },
      { type: "effectFail" },
      { type: "unknown" },
    ];
    return ["counter", "composition", "ordering"].flatMap((scenario) =>
      events.flatMap((event) => {
        const result = compareScenario({
          name: `Inspection ${scenario} ${event.type}`,
          scenario,
          steps: [
            { op: "inspect", listener: "events" },
            { op: "send", event },
          ],
        });
        return result.equal ? [] : [{ scenario, event, ...result }];
      }),
    );
  });
  expect(failures).toEqual([]);
});

test("inspection reports current initialization and outer macrosteps without observing dry runs", async ({
  page,
}) => {
  await page.goto("/");
  await expect(
    page.getByRole("button", { name: "Increment", exact: true }),
  ).toBeEnabled();
  const results = await page.evaluate(async () => {
    const rustPath = "/src/engines/rust.ts";
    const referencePath = "/src/engines/reference.ts";
    const { createRust } = await import(rustPath);
    const { createReference } = await import(referencePath);
    return [createRust, createReference].map((create) => {
      const trace: unknown[] = [];
      const engine = create("composition", (entry: unknown) =>
        trace.push(entry),
      );
      try {
        engine.send({ type: "inc", by: 4 });
        engine.subscribe((snapshot: unknown) =>
          trace.push({ kind: "observe", snapshot }),
        );
        engine.subscribeSelected("total", (value: number) =>
          trace.push({ kind: "select", value }),
        );
        engine.onAny((message: unknown) =>
          trace.push({ kind: "emit", message }),
        );
        const unsubscribe = engine.inspect((record: unknown) =>
          trace.push({ kind: "inspect", record }),
        );
        const afterInitial = trace.length;
        engine.can({ type: "add", price: 8 });
        engine.transition({ type: "add", price: 8 });
        const afterDryRuns = trace.length;
        engine.send({ type: "unknown" });
        engine.send({ type: "keep" });
        engine.send({ type: "same" });
        engine.send({ type: "add", price: 8 });
        unsubscribe();
        unsubscribe();
        engine.send({ type: "keep" });
        return { trace, afterInitial, afterDryRuns, final: engine.snapshot() };
      } finally {
        engine.dispose();
      }
    });
  });
  expect(results[0]).toEqual(results[1]);
  expect(results[0].afterInitial).toBe(1);
  expect(results[0].afterDryRuns).toBe(1);
  const inspections = results[0].trace.filter(
    (entry: any) => entry.kind === "inspect",
  ) as any[];
  expect(inspections.map((entry) => entry.record.event.type)).toEqual([
    "@xstate.init",
    "unknown",
    "keep",
    "same",
    "add",
  ]);
  expect(inspections[0].record.snapshot.context.count).toBe(4);
  expect(inspections.at(-1).record.snapshot.context).toEqual({
    count: 4,
    items: [8],
    total: 8,
  });
  expect(results[0].trace.slice(-4).map((entry: any) => entry.kind)).toEqual([
    "observe",
    "select",
    "inspect",
    "emit",
  ]);
});

test("live inspector additions removals and initial nested registration keep distinct handles", async ({
  page,
}) => {
  await page.goto("/");
  await expect(
    page.getByRole("button", { name: "Increment", exact: true }),
  ).toBeEnabled();
  const results = await page.evaluate(async () => {
    const rustPath = "/src/engines/rust.ts";
    const referencePath = "/src/engines/reference.ts";
    const { createRust } = await import(rustPath);
    const { createReference } = await import(referencePath);
    return [createRust, createReference].map((create) => {
      const engine = create("counter", () => {});
      const trace: string[] = [];
      let child = () => {};
      let tail = () => {};
      let added = false;
      try {
        const first = engine.inspect((record: any) => {
          trace.push(
            `first:${record.event.type}:${record.snapshot.context.count}`,
          );
          if (record.event.type === "@xstate.init" && !added) {
            added = true;
            child = engine.inspect((nested: any) => {
              trace.push(
                `child:${nested.event.type}:${nested.snapshot.context.count}`,
              );
            });
          }
          if (
            record.event.type === "inc" &&
            record.snapshot.context.count === 1
          ) {
            tail();
            child();
            child = engine.inspect((nested: any) => {
              trace.push(
                `added:${nested.event.type}:${nested.snapshot.context.count}`,
              );
            });
          }
        });
        tail = engine.inspect((record: any) => {
          trace.push(
            `tail:${record.event.type}:${record.snapshot.context.count}`,
          );
        });
        engine.send({ type: "inc", by: 1 });
        first();
        engine.send({ type: "inc", by: 1 });
        child();
        engine.send({ type: "inc", by: 1 });
        return { trace, count: engine.snapshot().context.count };
      } finally {
        engine.dispose();
      }
    });
  });
  expect(results[0]).toEqual(results[1]);
  expect(results[0].trace).toEqual([
    "first:@xstate.init:0",
    "child:@xstate.init:0",
    "tail:@xstate.init:0",
    "first:inc:1",
    "added:@xstate.init:1",
    "added:inc:1",
    "added:inc:2",
  ]);
  expect(results[0].count).toBe(3);
});

test("inspection transition errors preserve committed state with fail fast delivery", async ({
  page,
}) => {
  await page.goto("/");
  await expect(
    page.getByRole("button", { name: "Increment", exact: true }),
  ).toBeEnabled();
  const results = await page.evaluate(async () => {
    const rustPath = "/src/engines/rust.ts";
    const referencePath = "/src/engines/reference.ts";
    const { createRust } = await import(rustPath);
    const { createReference } = await import(referencePath);
    return [createRust, createReference].map((create) => {
      const trace: unknown[] = [];
      const engine = create("ordering", (entry: unknown) => trace.push(entry));
      const attempt = (run: () => void) => {
        try {
          run();
          return "no error";
        } catch (failure) {
          return failure instanceof Error ? failure.message : String(failure);
        }
      };
      try {
        const bad = engine.inspect((record: any) => {
          trace.push({
            kind: "bad",
            event: record.event.type,
            count: record.snapshot.context.count,
          });
          if (record.event.type !== "@xstate.init")
            throw new Error("inspection failed");
        });
        engine.inspect((record: any) =>
          trace.push({ kind: "tail", event: record.event.type }),
        );
        engine.subscribe((snapshot: any) =>
          trace.push({ kind: "observe", count: snapshot.context.count }),
        );
        trace.length = 0;
        const dispatchError = attempt(() => engine.send({ type: "start" }));
        const afterFailure = {
          trace: structuredClone(trace),
          snapshot: engine.snapshot(),
        };
        bad();
        trace.length = 0;
        const subscriber = engine.subscribe(() => {
          throw new Error("subscriber failed");
        });
        const subscriberError = attempt(() => engine.send({ type: "start" }));
        const afterSubscriberFailure = structuredClone(trace);
        subscriber();
        trace.length = 0;
        engine.send({ type: "start" });
        return {
          dispatchError,
          subscriberError,
          afterFailure,
          afterSubscriberFailure,
          recovered: trace,
        };
      } finally {
        engine.dispose();
      }
    });
  });
  expect(results[0]).toEqual(results[1]);
  expect(results[0]).toMatchObject({
    dispatchError: "inspection failed",
    subscriberError: "subscriber failed",
    afterFailure: {
      trace: [
        { kind: "observe", count: 1 },
        { kind: "bad", event: "start", count: 1 },
      ],
    },
    afterSubscriberFailure: [{ kind: "observe", count: 2 }],
  });
  expect(
    results[0].recovered.filter((entry: any) => entry.kind === "effect"),
  ).toHaveLength(2);
});

test("failed native inspection registration cleans up after synchronous reentry while upstream retains it", async ({
  page,
}) => {
  await page.goto("/");
  await expect(
    page.getByRole("button", { name: "Increment", exact: true }),
  ).toBeEnabled();
  const results = await page.evaluate(async () => {
    const rustPath = "/src/engines/rust.ts";
    const referencePath = "/src/engines/reference.ts";
    const { createRust } = await import(rustPath);
    const { createReference } = await import(referencePath);
    return [createRust, createReference].map((create) => {
      const engine = create("counter", () => {});
      const trace: string[] = [];
      let child = () => {};
      engine.inspect((record: any) => {
        trace.push(`valid:${record.event.type}:${record.snapshot.context.count}`);
      });
      let failure: { message: string; dispatchStatus: unknown } | undefined;
      try {
        try {
          engine.inspect((record: any) => {
            trace.push(
              `failed:${record.event.type}:${record.snapshot.context.count}`,
            );
            if (record.event.type === "@xstate.init") {
              child = engine.inspect((nested: any) => {
                trace.push(
                  `child:${nested.event.type}:${nested.snapshot.context.count}`,
                );
              });
              engine.send({ type: "inc", by: 1 });
              throw new Error("initial failed");
            }
          });
        } catch (error) {
          failure = {
            message: (error as Error).message,
            dispatchStatus: (error as { dispatchStatus?: string }).dispatchStatus ?? null,
          };
        }
        const initial = [...trace];
        trace.length = 0;
        engine.send({ type: "inc", by: 1 });
        child();
        const retained = [...trace];
        trace.length = 0;
        engine.send({ type: "inc", by: 1 });
        return { failure, initial, retained, detached: trace, count: engine.snapshot().context.count };
      } finally {
        engine.dispose();
      }
    });
  });
  expect(results[0].failure).toEqual({ message: "initial failed", dispatchStatus: null });
  expect(results[1].failure).toEqual(results[0].failure);
  expect(results[0].initial).toEqual(results[1].initial);
  expect(results[0].initial).toEqual([
    "valid:@xstate.init:0",
    "failed:@xstate.init:0",
    "child:@xstate.init:0",
    "valid:inc:1",
    "failed:inc:1",
    "child:inc:1",
  ]);
  expect(results[0].retained).toEqual(["valid:inc:2", "child:inc:2"]);
  expect(results[1].retained).toEqual(["valid:inc:2", "failed:inc:2", "child:inc:2"]);
  expect(results[0].detached).toEqual(["valid:inc:3"]);
  expect(results[1].detached).toEqual(["valid:inc:3", "failed:inc:3"]);
  expect(results.map((result) => result.count)).toEqual([3, 3]);
});
