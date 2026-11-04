import { expect, test } from "@playwright/test";

test("snapshot persistence skips stale outer writes after subscriber reentry", async ({
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
      let saved: string | null = null;
      const engine = create(
        { historyLimit: 4, persistence: { name: "nested", version: 1 } },
        (entry: unknown) => trace.push(entry),
        {
          getItem: () => null,
          setItem: (_name: string, value: string) => {
            saved = value;
            trace.push({ kind: "write", value: JSON.parse(value) });
          },
          removeItem: () => {
            saved = null;
          },
        },
      );
      engine.subscribe((snapshot: any) => {
        trace.push({ kind: "observe", count: snapshot.context.count });
        if (snapshot.context.count === 1) engine.send({ type: "inc", by: 10 });
      });
      engine.inspect((record: any) =>
        trace.push({
          kind: "inspect",
          event: record.event.type,
          count: record.snapshot.context.count,
        }),
      );
      trace.length = 0;
      try {
        engine.send({ type: "inc", by: 1 });
        return {
          trace,
          saved: JSON.parse(saved!),
          context: engine.snapshot().context,
          history: engine.history(),
        };
      } finally {
        engine.dispose();
      }
    });
  });
  expect(results[0]).toEqual(results[1]);
  expect(results[0].saved.context.count).toBe(11);
  expect(
    results[0].trace.filter((entry: any) => entry.kind === "write"),
  ).toHaveLength(1);
});

test("clearing during inspection prevents a stale outer save and preserves live state", async ({
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
      let saved: string | null = null;
      const engine = create(
        { historyLimit: 4, persistence: { name: "clear", version: 1 } },
        () => {},
        {
          getItem: () => null,
          setItem: (_name: string, value: string) => {
            saved = value;
            trace.push({ kind: "write", value: JSON.parse(value) });
          },
          removeItem: () => {
            saved = null;
            trace.push({ kind: "clear" });
          },
        },
      );
      engine.inspect((record: any) => {
        if (record.event.type === "inc" && record.snapshot.context.count === 1)
          engine.clearStorage();
      });
      try {
        engine.send({ type: "inc", by: 1 });
        const afterClear = {
          saved,
          count: engine.snapshot().context.count,
          history: engine.history(),
        };
        engine.send({ type: "inc", by: 1 });
        return {
          trace,
          afterClear,
          saved: JSON.parse(saved!),
          context: engine.snapshot().context,
        };
      } finally {
        engine.dispose();
      }
    });
  });
  expect(results[0]).toEqual(results[1]);
  expect(results[0].afterClear).toMatchObject({
    saved: null,
    count: 1,
    history: { past: 1 },
  });
  expect(results[0].trace.map((entry: any) => entry.kind)).toEqual([
    "clear",
    "write",
  ]);
  expect(results[0].saved.context.count).toBe(2);
});

test("automatic storage errors do not fail committed actions or suppress user effects", async ({
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
      let fail = true;
      const engine = create(
        {
          scenario: "composition",
          historyLimit: 3,
          persistence: { name: "failure", version: 1 },
        },
        (entry: unknown) => trace.push(entry),
        {
          getItem: () => null,
          setItem: () => {
            if (fail) throw new Error("write failed");
          },
          removeItem: () => {},
        },
      );
      engine.onAny((message: unknown) => trace.push({ kind: "emit", message }));
      try {
        engine.send({ type: "add", price: 8 });
        const failed = {
          trace: structuredClone(trace),
          error: engine.persistenceError(),
          context: engine.snapshot().context,
          history: engine.history(),
        };
        fail = false;
        engine.send({ type: "inc", by: 0 });
        return { failed, recovered: engine.persistenceError() };
      } finally {
        engine.dispose();
      }
    });
  });
  expect(results[0]).toEqual(results[1]);
  expect(results[0].failed.trace).toEqual([
    { kind: "persistError", message: "write failed" },
    { kind: "emit", message: { type: "receipt", total: 8 } },
  ]);
  expect(results[0].failed.context.total).toBe(8);
  expect(results[0].failed.history.past).toBe(1);
  expect(results[0].recovered).toBeNull();
});

test("native persistence rejects corrupt versions and supports explicit retry and empty-control policy", async ({
  page,
}) => {
  await page.goto("/");
  await expect(
    page.getByRole("button", { name: "Increment", exact: true }),
  ).toBeEnabled();
  const result = await page.evaluate(async () => {
    const path = "/src/engines/rust-controls.ts";
    const { createRustControls } = await import(path);
    const configuration = {
      historyLimit: 3,
      initialCount: 5,
      persistence: { name: "native", version: 1 },
    };
    const failures = [
      "{",
      JSON.stringify({
        context: { count: 8, items: [], total: 0 },
        version: 2,
      }),
    ].map((stored) => {
      let writes = 0;
      try {
        const engine = createRustControls(configuration, () => {}, {
          getItem: () => stored,
          setItem: () => {
            writes++;
          },
          removeItem: () => {},
        });
        engine.dispose();
        return { failed: false, writes };
      } catch (failure) {
        return { failed: true, message: (failure as Error).message, writes };
      }
    });
    let writeFailure = true;
    let saved: string | null = null;
    const engine = createRustControls(configuration, () => {}, {
      getItem: () => null,
      setItem: (_name: string, value: string) => {
        if (writeFailure) throw new Error("write failed");
        saved = value;
      },
      removeItem: () => {
        saved = null;
      },
    });
    try {
      let notifications = 0;
      engine.subscribe(() => notifications++);
      const initial = engine.snapshot();
      engine.undo();
      engine.redo();
      const emptyControls = {
        notifications,
        same: initial === engine.snapshot(),
        history: engine.history(),
      };
      engine.send({ type: "inc", by: 2 });
      const error = engine.persistenceError();
      writeFailure = false;
      engine.save();
      const retried = {
        error: engine.persistenceError(),
        saved: JSON.parse(saved!),
      };
      const beforeClear = engine.history();
      engine.clearStorage();
      return {
        failures,
        emptyControls,
        error,
        retried,
        cleared: saved,
        beforeClear,
        afterClear: engine.history(),
        count: engine.snapshot().context.count,
      };
    } finally {
      engine.dispose();
    }
  });
  expect(
    result.failures.every(
      (failure: any) => failure.failed && failure.writes === 0,
    ),
  ).toBe(true);
  expect(result.failures[1].message).toContain("version");
  expect(result.emptyControls).toEqual({
    notifications: 0,
    same: true,
    history: { past: 0, future: 0, limit: 3 },
  });
  expect(result.error).toBe("write failed");
  expect(result.retried).toMatchObject({
    error: null,
    saved: { context: { count: 7 }, version: 1 },
  });
  expect(result.cleared).toBeNull();
  expect(result.beforeClear).toEqual(result.afterClear);
  expect(result.count).toBe(7);
});

test("native synchronous localStorage snapshots survive recreation without persisting history", async ({
  page,
}) => {
  await page.goto("/");
  await expect(
    page.getByRole("button", { name: "Increment", exact: true }),
  ).toBeEnabled();
  await page.evaluate(async () => {
    const path = "/src/engines/rust-controls.ts";
    const { createRustControls } = await import(path);
    localStorage.removeItem("rstate-local-fixture");
    const engine = createRustControls(
      {
        scenario: "composition",
        initialCount: 2,
        historyLimit: 3,
        persistence: { name: "rstate-local-fixture", version: 1 },
      },
      () => {},
      localStorage,
    );
    try {
      engine.send({ type: "inc", by: 7 });
      engine.send({ type: "add", price: 8 });
    } finally {
      engine.dispose();
    }
  });
  await page.reload();
  await expect(
    page.getByRole("button", { name: "Increment", exact: true }),
  ).toBeEnabled();
  const result = await page.evaluate(async () => {
    const path = "/src/engines/rust-controls.ts";
    const { createRustControls } = await import(path);
    const engine = createRustControls(
      {
        scenario: "composition",
        initialCount: 2,
        historyLimit: 3,
        persistence: { name: "rstate-local-fixture", version: 1 },
      },
      () => {},
      localStorage,
    );
    try {
      const initial = engine.initialSnapshot();
      const history = engine.history();
      engine.reset();
      const reset = engine.snapshot().context;
      engine.undo();
      const undone = engine.snapshot().context;
      engine.clearStorage();
      return {
        initial,
        history,
        reset,
        undone,
        cleared: localStorage.getItem("rstate-local-fixture"),
      };
    } finally {
      engine.dispose();
      localStorage.removeItem("rstate-local-fixture");
    }
  });
  expect(result.initial.context).toEqual({ count: 9, items: [8], total: 8 });
  expect(result.history).toEqual({ past: 0, future: 0, limit: 3 });
  expect(result.reset).toEqual({ count: 2, items: [], total: 0 });
  expect(result.undone).toEqual({ count: 9, items: [8], total: 8 });
  expect(result.cleared).toBeNull();
});

test("native persistence bookkeeping keeps unchanged snapshots while upstream metadata reallocates", async ({
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
      let writes = 0;
      let notifications = 0;
      const engine = create(
        { persistence: { name: "noop", version: 1 } },
        () => {},
        {
          getItem: () => null,
          setItem: () => {
            writes++;
          },
          removeItem: () => {},
        },
      );
      engine.subscribe(() => notifications++);
      try {
        const before = engine.snapshot();
        engine.send({ type: "keep" });
        const after = engine.snapshot();
        return {
          writes,
          notifications,
          sameSnapshot: before.snapshotId === after.snapshotId,
          sameContext: before.contextId === after.contextId,
        };
      } finally {
        engine.dispose();
      }
    });
  });
  expect(results[0]).toEqual({
    writes: 1,
    notifications: 0,
    sameSnapshot: true,
    sameContext: true,
  });
  expect(results[1]).toEqual({
    writes: 1,
    notifications: 1,
    sameSnapshot: false,
    sameContext: true,
  });
});
