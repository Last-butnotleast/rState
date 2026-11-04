import { expect, test } from "@playwright/test";

test("snapshot undo and redo restore state without replaying reducers or effects", async ({
  page,
}) => {
  await page.goto("/");
  await expect(
    page.getByRole("button", { name: "Increment", exact: true }),
  ).toBeEnabled();
  const results = await page.evaluate(async () => {
    const rustPath = "/src/engines/rust-controls.ts";
    const referencePath = "/src/engines/reference-controls.ts";
    const { createRustControls } = await import(rustPath);
    const { createReferenceControls } = await import(referencePath);
    return [createRustControls, createReferenceControls].map((create) => {
      const trace: unknown[] = [];
      const engine = create(
        { scenario: "ordering", historyLimit: 4 },
        (entry: unknown) => trace.push(entry),
      );
      try {
        engine.send({ type: "start" });
        const after = {
          count: engine.snapshot().context.count,
          history: engine.history(),
          effects: trace.length,
        };
        engine.undo();
        const undone = {
          count: engine.snapshot().context.count,
          history: engine.history(),
          effects: trace.length,
        };
        engine.redo();
        const redone = {
          count: engine.snapshot().context.count,
          history: engine.history(),
          effects: trace.length,
        };
        let error = "";
        try {
          engine.send({ type: "effectFail" });
        } catch (failure) {
          error = (failure as Error).message;
        }
        const failed = {
          count: engine.snapshot().context.count,
          history: engine.history(),
          error,
        };
        engine.undo();
        return {
          after,
          undone,
          redone,
          failed,
          recovered: engine.snapshot().context.count,
        };
      } finally {
        engine.dispose();
      }
    });
  });
  expect(results[0]).toEqual(results[1]);
  expect(results[0]).toMatchObject({
    after: { count: 11, history: { past: 2 }, effects: 2 },
    undone: { count: 1, history: { past: 1, future: 1 }, effects: 2 },
    redone: { count: 11, history: { past: 2, future: 0 }, effects: 2 },
    failed: { count: 12, error: "effect failed" },
    recovered: 11,
  });
});

test("reset inspection preserves an outer snapshot after subscriber reentry", async ({
  page,
}) => {
  await page.goto("/");
  await expect(
    page.getByRole("button", { name: "Increment", exact: true }),
  ).toBeEnabled();
  const results = await page.evaluate(async () => {
    const rustPath = "/src/engines/rust-controls.ts";
    const referencePath = "/src/engines/reference-controls.ts";
    const { createRustControls } = await import(rustPath);
    const { createReferenceControls } = await import(referencePath);
    return [createRustControls, createReferenceControls].map((create) => {
      const engine = create({ historyLimit: 4 }, () => {});
      const trace: unknown[] = [];
      engine.subscribe((snapshot: any) => {
        if (snapshot.context.count === 1) engine.reset();
      });
      engine.inspect((record: any) =>
        trace.push({
          event: record.event.type,
          snapshot: record.snapshot.context.count,
          current: engine.snapshot().context.count,
          past: record.snapshot.history.past,
        }),
      );
      try {
        engine.send({ type: "inc", by: 1 });
        return {
          trace,
          history: engine.history(),
          context: engine.snapshot().context,
        };
      } finally {
        engine.dispose();
      }
    });
  });
  expect(results[0]).toEqual(results[1]);
  expect(results[0].trace).toEqual([
    { event: "@xstate.init", snapshot: 0, current: 0, past: 0 },
    { event: "reset", snapshot: 0, current: 0, past: 2 },
    { event: "inc", snapshot: 1, current: 0, past: 1 },
  ]);
});

test("native history preserves domain availability and empty controls do not publish", async ({
  page,
}) => {
  await page.goto("/");
  await expect(
    page.getByRole("button", { name: "Increment", exact: true }),
  ).toBeEnabled();
  const result = await page.evaluate(async () => {
    const rustPath = "/src/engines/rust-controls.ts";
    const referencePath = "/src/engines/reference-controls.ts";
    const { createRustControls } = await import(rustPath);
    const { createReferenceControls } = await import(referencePath);
    return [createRustControls, createReferenceControls].map((create) => {
      const engine = create({ historyLimit: 2 }, () => {});
      let notifications = 0;
      engine.subscribe(() => notifications++);
      try {
        const initial = engine.snapshot();
        const allowed = engine.can({ type: "keep" });
        engine.transition({ type: "inc", by: 3 });
        const afterDryRuns = engine.history();
        engine.undo();
        engine.redo();
        return {
          allowed,
          afterDryRuns,
          notifications,
          same: initial === engine.snapshot(),
          history: engine.history(),
        };
      } finally {
        engine.dispose();
      }
    });
  });
  expect(result[0]).toEqual({
    allowed: false,
    afterDryRuns: { past: 0, future: 0, limit: 2 },
    notifications: 0,
    same: true,
    history: { past: 0, future: 0, limit: 2 },
  });
  expect(result[1].allowed).toBe(true);
  expect(result[1].notifications).toBe(0);
});
