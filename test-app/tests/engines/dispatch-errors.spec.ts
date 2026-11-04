import { expect, test } from "@playwright/test";

test("native dispatch errors identify the call's commit boundary including unchanged snapshots and nested failures", async ({
  page,
}) => {
  await page.goto("/");
  await expect(
    page.getByRole("button", { name: "Increment", exact: true }),
  ).toBeEnabled();
  const results = await page.evaluate(async () => {
    const path = "/src/engines/rust.ts";
    const { createRust } = await import(path);
    const capture = (
      scenario: string,
      event: { type: string; by?: number },
      observe?: (engine: any) => void,
    ) => {
      const trace: unknown[] = [];
      const engine = createRust(scenario, (entry: unknown) => trace.push(entry));
      try {
        observe?.(engine);
        const before = engine.snapshot();
        let error: { message: string; dispatchStatus: unknown } | undefined;
        try {
          engine.send(event);
        } catch (failure) {
          error = {
            message: (failure as Error).message,
            dispatchStatus: (failure as { dispatchStatus?: string }).dispatchStatus ?? null,
          };
        }
        return {
          error,
          count: engine.snapshot().context.count,
          sameSnapshot: engine.snapshot() === before,
          trace,
        };
      } finally {
        engine.dispose();
      }
    };
    return {
      reducer: capture("counter", { type: "fail" }),
      subscriber: capture("counter", { type: "inc", by: 1 }, (engine) => {
        engine.subscribe(() => {
          throw new Error("subscriber failed");
        });
      }),
      inspector: capture("ordering", { type: "start" }, (engine) => {
        engine.inspect((record: any) => {
          if (record.event.type !== "@xstate.init") {
            throw new Error("inspector failed");
          }
        });
      }),
      effect: capture("ordering", { type: "effectFail" }),
      unchangedEmission: capture("composition", { type: "receipt" }, (engine) => {
        engine.onAny(() => {
          throw new Error("emitted listener failed");
        });
      }),
      nested: capture("ordering", { type: "start" }, (engine) => {
        engine.subscribe((snapshot: any) => {
          if (snapshot.context.count === 11) {
            throw new Error("nested subscriber failed");
          }
        });
      }),
    };
  });
  expect(results).toEqual({
    reducer: {
      error: { message: "reducer failed", dispatchStatus: "notCommitted" },
      count: 0,
      sameSnapshot: true,
      trace: [],
    },
    subscriber: {
      error: { message: "subscriber failed", dispatchStatus: "committed" },
      count: 1,
      sameSnapshot: false,
      trace: [],
    },
    inspector: {
      error: { message: "inspector failed", dispatchStatus: "committed" },
      count: 1,
      sameSnapshot: false,
      trace: [],
    },
    effect: {
      error: { message: "effect failed", dispatchStatus: "committed" },
      count: 1,
      sameSnapshot: false,
      trace: [],
    },
    unchangedEmission: {
      error: { message: "emitted listener failed", dispatchStatus: "committed" },
      count: 0,
      sameSnapshot: true,
      trace: [],
    },
    nested: {
      error: { message: "nested subscriber failed", dispatchStatus: "committed" },
      count: 11,
      sameSnapshot: false,
      trace: [{ kind: "effect", label: "before", count: 1 }],
    },
  });
});

test("native validation and control failures expose dispatch status while pure query errors do not", async ({
  page,
}) => {
  await page.goto("/");
  await expect(
    page.getByRole("button", { name: "Increment", exact: true }),
  ).toBeEnabled();
  const results = await page.evaluate(async () => {
    const rustPath = "/src/engines/rust.ts";
    const controlsPath = "/src/engines/rust-controls.ts";
    const validationPath = "/src/engines/rust-validation.ts";
    const { createRust } = await import(rustPath);
    const { createRustControls } = await import(controlsPath);
    const { createRustValidation } = await import(validationPath);
    const attempt = (run: () => unknown) => {
      try {
        run();
        return null;
      } catch (failure) {
        const error = failure as {
          message: string;
          dispatchStatus?: string;
          reason?: string;
        };
        return {
          message: error.message,
          dispatchStatus: error.dispatchStatus ?? null,
          reason: error.reason ?? null,
        };
      }
    };
    const counter = createRust("counter", () => {});
    const validated = createRustValidation({}, () => {});
    const controls = createRustControls({ initialCount: 3, historyLimit: 2 }, () => {});
    try {
      const transition = attempt(() => counter.transition({ type: "fail" }));
      const availability = attempt(() => counter.can({ type: "fail" }));
      const validatedInitial = validated.snapshot();
      const rejectedEvent = attempt(() => validated.send({ type: "inc", by: 6 }));
      const rejectedQuery = attempt(() => validated.transition({ type: "inc", by: 6 }));
      const rejectedReset = attempt(() => validated.resetWithCount(-1));
      const unchanged = validated.snapshot() === validatedInitial;
      controls.send({ type: "inc", by: 1 });
      const unsubscribe = controls.subscribe(() => {
        throw new Error("control subscriber failed");
      });
      const reset = attempt(() => controls.reset());
      const afterReset = controls.snapshot().context.count;
      const undo = attempt(() => controls.undo());
      const afterUndo = controls.snapshot().context.count;
      const redo = attempt(() => controls.redo());
      const afterRedo = controls.snapshot().context.count;
      unsubscribe();
      return {
        transition,
        availability,
        rejectedEvent,
        rejectedQuery,
        rejectedReset,
        unchanged,
        reset,
        undo,
        redo,
        counts: [afterReset, afterUndo, afterRedo],
      };
    } finally {
      counter.dispose();
      validated.dispose();
      controls.dispose();
    }
  });
  expect(results.transition).toEqual({
    message: "reducer failed",
    dispatchStatus: null,
    reason: null,
  });
  expect(results.availability).toEqual(results.transition);
  expect(results.rejectedEvent).toMatchObject({
    dispatchStatus: "notCommitted",
    reason: "invalidEvent",
  });
  expect(results.rejectedQuery).toMatchObject({
    dispatchStatus: null,
    reason: "invalidEvent",
  });
  expect(results.rejectedReset).toMatchObject({
    dispatchStatus: "notCommitted",
    reason: "invalidContext",
  });
  expect(results.unchanged).toBe(true);
  for (const failure of [results.reset, results.undo, results.redo]) {
    expect(failure).toEqual({
      message: "control subscriber failed",
      dispatchStatus: "committed",
      reason: null,
    });
  }
  expect(results.counts).toEqual([3, 4, 3]);
});
