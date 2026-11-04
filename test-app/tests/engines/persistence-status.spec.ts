import { expect, test } from "@playwright/test";

test("native persistence status observes failure recovery and clear without changing snapshots", async ({
  page,
}) => {
  await page.goto("/");
  await expect(page.getByRole("button", { name: "Increment", exact: true })).toBeEnabled();
  const result = await page.evaluate(async () => {
    const path = "/src/generated/rstate_wasm.js";
    const { FixtureControls } = await import(path);
    const statuses: unknown[] = [];
    const diagnostics: unknown[] = [];
    const storageErrors: unknown[] = [];
    let writeFailure: string | null = "first write failed";
    let clearFailure = false;
    let saved: string | null = null;
    let writes = 0;
    let stateNotifications = 0;
    const controls = new FixtureControls(
      JSON.stringify({ initialCount: 4, persistence: { name: "status", version: 1 } }),
      () => {},
      () => null,
      (_name: string, value: string) => {
        writes += 1;
        if (writeFailure) throw new Error(writeFailure);
        saved = value;
      },
      () => {
        if (clearFailure) throw new Error("clear failed");
        saved = null;
      },
      (serialized: string) => storageErrors.push(JSON.parse(serialized)),
    );
    const store = controls.base();
    store.subscribe(() => { stateNotifications += 1; });
    const subscription = store.subscribe_persistence(
      (serialized: string) => statuses.push(JSON.parse(serialized)),
      (serialized: string) => diagnostics.push(JSON.parse(serialized)),
    );
    const initial = JSON.parse(controls.persistence_status());
    const initialNotifications = statuses.length;
    const attempt = (operation: () => void) => {
      try {
        operation();
        return null;
      } catch (failure) {
        return failure instanceof Error ? failure.message : String(failure);
      }
    };
    try {
      store.send(JSON.stringify({ type: "inc", by: 1 }));
      const committed = store.snapshot();
      writeFailure = "replacement write failed";
      const saveError = attempt(() => controls.save());
      writeFailure = null;
      controls.save();
      clearFailure = true;
      const clearError = attempt(() => controls.clear_storage());
      clearFailure = false;
      controls.clear_storage();
      const afterClear = JSON.parse(controls.persistence_status());
      const clearedStorage = saved;
      const observed = structuredClone(statuses);
      store.unsubscribe(subscription);
      controls.save();
      const afterUnsubscribe = statuses.length;
      store.subscribe_persistence(
        (serialized: string) => statuses.push(JSON.parse(serialized)),
        (serialized: string) => diagnostics.push(JSON.parse(serialized)),
      );
      store.dispose();
      controls.save();
      return {
        initial,
        initialNotifications,
        observed,
        afterClear,
        clearedStorage,
        saveError,
        clearError,
        storageErrors,
        diagnostics,
        writes,
        stateNotifications,
        stableSnapshot: committed === store.snapshot(),
        afterUnsubscribe,
        afterDispose: statuses.length,
      };
    } finally {
      store.dispose();
      store.free();
      controls.free();
    }
  });
  expect(result.initial).toEqual({ sequence: 0, activity: null, outcome: null, error: null });
  expect(result.initialNotifications).toBe(0);
  expect(result.observed).toEqual([
    { sequence: 1, activity: "automaticSave", outcome: "failed", error: "first write failed" },
    { sequence: 2, activity: "save", outcome: "failed", error: "replacement write failed" },
    { sequence: 3, activity: "save", outcome: "succeeded", error: null },
    { sequence: 4, activity: "clear", outcome: "failed", error: "clear failed" },
    { sequence: 5, activity: "clear", outcome: "succeeded", error: null },
  ]);
  expect(result.afterClear).toEqual(result.observed[4]);
  expect(result.clearedStorage).toBeNull();
  expect(result.saveError).toContain("replacement write failed");
  expect(result.clearError).toContain("clear failed");
  expect(result.storageErrors).toEqual(["first write failed", "replacement write failed", "clear failed"]);
  expect(result.diagnostics).toEqual([]);
  expect(result.writes).toBe(5);
  expect(result.stateNotifications).toBe(1);
  expect(result.stableSnapshot).toBe(true);
  expect(result.afterUnsubscribe).toBe(5);
  expect(result.afterDispose).toBe(5);
});

test("native persistence feedback can retry synchronously and later observers see recovered status", async ({
  page,
}) => {
  await page.goto("/");
  await expect(page.getByRole("button", { name: "Increment", exact: true })).toBeEnabled();
  const result = await page.evaluate(async () => {
    const path = "/src/generated/rstate_wasm.js";
    const { FixtureControls } = await import(path);
    const trace: string[] = [];
    let fail = true;
    let writes = 0;
    let saved: string | null = null;
    let feedbackSnapshot: string | null = null;
    const controls = new FixtureControls(
      JSON.stringify({ persistence: { name: "status-retry", version: 1 } }),
      () => {},
      () => null,
      (_name: string, value: string) => {
        writes += 1;
        if (fail) throw new Error("retry write failed");
        saved = value;
      },
      () => {},
      () => {},
    );
    const store = controls.base();
    const diagnostics = (serialized: string) => trace.push(`diagnostic:${JSON.parse(serialized)}`);
    store.subscribe_persistence((serialized: string) => {
      const status = JSON.parse(serialized);
      trace.push(`first:${status.outcome}`);
      feedbackSnapshot = store.snapshot();
      fail = false;
      controls.save();
      trace.push(`first-return:${JSON.parse(controls.persistence_status()).outcome}`);
    }, diagnostics);
    store.subscribe_persistence((serialized: string) => {
      trace.push(`tail:${JSON.parse(serialized).outcome}`);
    }, diagnostics);
    try {
      store.send(JSON.stringify({ type: "inc", by: 1 }));
      return {
        trace,
        writes,
        saved: JSON.parse(saved!),
        status: JSON.parse(controls.persistence_status()),
        stableSnapshot: feedbackSnapshot === store.snapshot(),
      };
    } finally {
      store.dispose();
      store.free();
      controls.free();
    }
  });
  expect(result.trace).toEqual(["first:failed", "tail:succeeded", "first-return:succeeded"]);
  expect(result.writes).toBe(2);
  expect(result.saved.context.count).toBe(1);
  expect(result.status).toEqual({ sequence: 2, activity: "save", outcome: "succeeded", error: null });
  expect(result.stableSnapshot).toBe(true);
});

test("native persistence feedback diagnostics do not become storage failures or skip emissions", async ({
  page,
}) => {
  await page.goto("/");
  await expect(page.getByRole("button", { name: "Increment", exact: true })).toBeEnabled();
  const result = await page.evaluate(async () => {
    const path = "/src/generated/rstate_wasm.js";
    const { FixtureControls } = await import(path);
    const trace: string[] = [];
    const storageErrors: unknown[] = [];
    const controls = new FixtureControls(
      JSON.stringify({ scenario: "composition", persistence: { name: "status-diagnostics", version: 1 } }),
      () => {},
      () => null,
      () => {},
      () => {},
      (serialized: string) => storageErrors.push(JSON.parse(serialized)),
    );
    const store = controls.base();
    const failedListener = store.subscribe_persistence(() => {
      throw new Error("feedback failed");
    }, (serialized: string) => trace.push(`diagnostic:${JSON.parse(serialized)}`));
    store.subscribe_persistence((serialized: string) => {
      trace.push(`status:${JSON.parse(serialized).outcome}`);
    }, () => {});
    store.on_any((serialized: string) => trace.push(`emit:${JSON.parse(serialized).total}`));
    try {
      store.send(JSON.stringify({ type: "add", price: 8 }));
      const first = [...trace];
      store.unsubscribe(failedListener);
      store.send(JSON.stringify({ type: "add", price: 0 }));
      const afterUnsubscribe = [...trace];
      const snapshot = store.snapshot();
      store.dispose();
      controls.save();
      return {
        first,
        afterUnsubscribe,
        afterDispose: trace,
        storageErrors,
        status: JSON.parse(controls.persistence_status()),
        stableSnapshot: snapshot === store.snapshot(),
        total: JSON.parse(store.snapshot()).context.total,
      };
    } finally {
      store.dispose();
      store.free();
      controls.free();
    }
  });
  expect(result.first).toEqual(["diagnostic:feedback failed", "status:succeeded", "emit:8"]);
  expect(result.afterUnsubscribe).toEqual([...result.first, "status:succeeded", "emit:8"]);
  expect(result.afterDispose).toEqual(result.afterUnsubscribe);
  expect(result.storageErrors).toEqual([]);
  expect(result.status).toEqual({ sequence: 3, activity: "save", outcome: "succeeded", error: null });
  expect(result.stableSnapshot).toBe(true);
  expect(result.total).toBe(8);
});
