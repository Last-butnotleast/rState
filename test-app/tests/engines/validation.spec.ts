import { expect, test } from "@playwright/test";

test("schema metadata is inert and validation construction is explicit", async ({
  page,
}) => {
  await page.goto("/");
  await expect(
    page.getByRole("button", { name: "Increment", exact: true }),
  ).toBeEnabled();
  const results = await page.evaluate(async () => {
    const rustPath = "/src/engines/rust-validation.ts";
    const sourcePath = "/src/engines/reference-validation.ts";
    const runnerPath = "/src/traces/validation.ts";
    const [
      { createRustValidation },
      { createReferenceValidation },
      { runValidation },
    ] = await Promise.all([
      import(rustPath),
      import(sourcePath),
      import(runnerPath),
    ]);
    return [createRustValidation, createReferenceValidation].map((create) => [
      runValidation(
        {
          name: "Metadata only",
          configuration: { runtime: false, initialCount: -2 },
          steps: [
            { op: "metadata" },
            { op: "send", event: { type: "inc", by: -2 } },
            { op: "snapshot" },
          ],
        },
        create,
      ),
      runValidation(
        {
          name: "No schema",
          configuration: { noSchemas: true, initialCount: -2 },
          steps: [
            { op: "metadata" },
            { op: "send", event: { type: "inc", by: -2 } },
            { op: "snapshot" },
          ],
        },
        create,
      ),
      runValidation(
        {
          name: "Rejected construction",
          configuration: { initialCount: -1 },
          steps: [],
        },
        create,
      ),
    ]);
  });
  expect(results[0]).toEqual(results[1]);
  expect(results[0][0].some((entry: any) => entry.kind === "validate")).toBe(
    false,
  );
  expect(results[0][1].some((entry: any) => entry.kind === "validate")).toBe(
    false,
  );
  expect(results[0][0].at(-1)).toMatchObject({
    result: { context: { count: -4 } },
  });
  expect(results[0][2].at(-1)).toEqual({
    kind: "createError",
    reason: "invalidContext",
    eventType: null,
    issues: [{ message: "count must be between 0 and 10", path: ["count"] }],
  });
});

test("upstream schema success outputs are ignored and values keep their identities", async ({
  page,
}) => {
  await page.goto("/");
  await expect(
    page.getByRole("button", { name: "Increment", exact: true }),
  ).toBeEnabled();
  const results = await page.evaluate(async () => {
    const rustPath = "/src/engines/rust-validation.ts";
    const sourcePath = "/src/engines/reference-validation.ts";
    const runnerPath = "/src/traces/validation.ts";
    const [
      { createRustValidation },
      { createReferenceValidation },
      { runValidation },
    ] = await Promise.all([
      import(rustPath),
      import(sourcePath),
      import(runnerPath),
    ]);
    return [createRustValidation, createReferenceValidation].map((create) =>
      runValidation(
        {
          name: "Ignored success values",
          configuration: { alteredOutput: true },
          steps: [
            { op: "subscribe" },
            { op: "inspect" },
            { op: "send", event: { type: "inc", by: 2 } },
            { op: "send", event: { type: "add", price: 2 } },
            { op: "send", event: { type: "same" } },
            { op: "snapshot" },
          ],
        },
        create,
      ),
    );
  });
  expect(results[0]).toEqual(results[1]);
  expect(results[0].at(-1)).toMatchObject({
    result: { context: { count: 2, items: [2], total: 2 } },
  });
  expect(results[0].filter((entry: any) => entry.kind === "emit")).toEqual([
    { kind: "emit", message: { type: "receipt", total: 2 } },
  ]);
  const snapshots = results[0]
    .filter((entry: any) => entry.kind === "inspect")
    .map((entry: any) => entry.snapshot);
  expect(snapshots.at(-1)).toEqual(snapshots.at(-2));
});

test("validation only checks final macrosteps and collected effects before publication", async ({
  page,
}) => {
  await page.goto("/");
  await expect(
    page.getByRole("button", { name: "Increment", exact: true }),
  ).toBeEnabled();
  const results = await page.evaluate(async () => {
    const rustPath = "/src/engines/rust-validation.ts";
    const sourcePath = "/src/engines/reference-validation.ts";
    const runnerPath = "/src/traces/validation.ts";
    const [
      { createRustValidation },
      { createReferenceValidation },
      { runValidation },
    ] = await Promise.all([
      import(rustPath),
      import(sourcePath),
      import(runnerPath),
    ]);
    return [createRustValidation, createReferenceValidation].map((create) => [
      runValidation(
        {
          name: "Intermediate context and internal payload bypass",
          configuration: {},
          steps: [
            { op: "subscribe" },
            { op: "inspect" },
            { op: "send", event: { type: "add", price: 2 } },
            { op: "snapshot" },
          ],
        },
        create,
      ),
      runValidation(
        {
          name: "Rejected macrostep suppresses functions and emissions",
          configuration: { initialCount: 10 },
          steps: [
            { op: "subscribe" },
            { op: "inspect" },
            { op: "send", event: { type: "start" } },
            { op: "snapshot" },
          ],
        },
        create,
      ),
      runValidation(
        {
          name: "Accepted functions and emissions",
          configuration: {},
          steps: [
            { op: "subscribe" },
            { op: "inspect" },
            { op: "send", event: { type: "start" } },
          ],
        },
        create,
      ),
    ]);
  });
  expect(results[0]).toEqual(results[1]);
  const eventChecks = results[0][0].filter(
    (entry: any) => entry.kind === "validate" && entry.target === "event",
  );
  expect(eventChecks.map((entry: any) => entry.name)).toEqual(["add"]);
  expect(
    results[0][0].filter((entry: any) => entry.kind === "observe"),
  ).toHaveLength(1);
  expect(
    results[0][1].some((entry: any) =>
      ["effect", "emit", "observe"].includes(entry.kind),
    ),
  ).toBe(false);
  expect(results[0][1].at(-1)).toMatchObject({
    result: { context: { count: 10 } },
  });
  expect(
    results[0][2]
      .filter((entry: any) =>
        ["observe", "inspect", "effect", "emit"].includes(entry.kind),
      )
      .slice(-4)
      .map((entry: any) => entry.kind),
  ).toEqual(["observe", "inspect", "effect", "emit"]);
});

test("validation options and unknown policies match typed upstream cases", async ({
  page,
}) => {
  await page.goto("/");
  await expect(
    page.getByRole("button", { name: "Increment", exact: true }),
  ).toBeEnabled();
  const results = await page.evaluate(async () => {
    const rustPath = "/src/engines/rust-validation.ts";
    const sourcePath = "/src/engines/reference-validation.ts";
    const runnerPath = "/src/traces/validation.ts";
    const [
      { createRustValidation },
      { createReferenceValidation },
      { runValidation },
    ] = await Promise.all([
      import(rustPath),
      import(sourcePath),
      import(runnerPath),
    ]);
    const configurations = [
      { context: false, events: false },
      { unknownEvents: "ignore" },
      { missingReceipt: true },
      { missingReceipt: true, unknownEmitted: "ignore" },
      { emitted: false },
    ];
    return [createRustValidation, createReferenceValidation].map((create) =>
      configurations.map((configuration) =>
        runValidation(
          {
            name: "Validation options",
            configuration,
            steps: [
              { op: "send", event: { type: "inc", by: -1 } },
              { op: "send", event: { type: "unknown" } },
              { op: "send", event: { type: "add", price: 2 } },
              { op: "snapshot" },
            ],
          },
          create,
        ),
      ),
    );
  });
  expect(results[0]).toEqual(results[1]);
  expect(
    results[0][2].find((entry: any) => entry.reason === "unknownEmitted"),
  ).toBeTruthy();
  expect(
    results[0][3].some((entry: any) => entry.reason === "unknownEmitted"),
  ).toBe(false);
  expect(
    results[0][1].some((entry: any) => entry.reason === "unknownEvent"),
  ).toBe(false);
});

test("rejected validation leaves history intact and validates restored control state", async ({
  page,
}) => {
  await page.goto("/");
  await expect(
    page.getByRole("button", { name: "Increment", exact: true }),
  ).toBeEnabled();
  const results = await page.evaluate(async () => {
    const rustPath = "/src/engines/rust-validation.ts";
    const sourcePath = "/src/engines/reference-validation.ts";
    const runnerPath = "/src/traces/validation.ts";
    const [
      { createRustValidation },
      { createReferenceValidation },
      { runValidation },
    ] = await Promise.all([
      import(rustPath),
      import(sourcePath),
      import(runnerPath),
    ]);
    return [createRustValidation, createReferenceValidation].map((create) =>
      runValidation(
        {
          name: "Validated history controls",
          configuration: { historyLimit: 3 },
          steps: [
            { op: "subscribe" },
            { op: "inspect" },
            { op: "send", event: { type: "inc", by: 2 } },
            { op: "send", event: { type: "inc", by: -1 } },
            { op: "history" },
            { op: "resetWithCount", count: -1 },
            { op: "history" },
            { op: "reset" },
            { op: "undo" },
            { op: "redo" },
            { op: "snapshot" },
          ],
        },
        create,
      ),
    );
  });
  expect(results[0]).toEqual(results[1]);
  expect(
    results[0]
      .filter((entry: any) => entry.op === "history" && entry.kind === "return")
      .map((entry: any) => entry.result),
  ).toEqual([
    { past: 1, future: 0, limit: 3 },
    { past: 1, future: 0, limit: 3 },
  ]);
  expect(results[0].at(-1)).toMatchObject({
    result: {
      context: { count: 0 },
      history: { past: 2, future: 0, limit: 3 },
    },
  });
});

test("validated persistence rejects invalid defaults or hydration without writing", async ({
  page,
}) => {
  await page.goto("/");
  await expect(
    page.getByRole("button", { name: "Increment", exact: true }),
  ).toBeEnabled();
  const result = await page.evaluate(async () => {
    const rustPath = "/src/engines/rust-validation.ts";
    const sourcePath = "/src/engines/reference-validation.ts";
    const helpersPath = "/src/engines/validation.ts";
    const [
      { createRustValidation },
      { createReferenceValidation },
      { validationFailure },
    ] = await Promise.all([
      import(rustPath),
      import(sourcePath),
      import(helpersPath),
    ]);
    const run = (create: any, initial: number, loaded: number) => {
      let stored = JSON.stringify({
        context: { count: loaded, items: [], total: 0 },
        version: 1,
      });
      const before = stored;
      const calls: string[] = [];
      let engine;
      let failure;
      try {
        engine = create(
          {
            initialCount: initial,
            historyLimit: 3,
            persistence: { name: "validation", version: 1 },
          },
          () => {},
          {
            getItem: () => {
              calls.push("read");
              return stored;
            },
            setItem: (_name: string, bytes: string) => {
              calls.push("write");
              stored = bytes;
            },
            removeItem: () => {
              calls.push("remove");
              stored = "";
            },
          },
        );
        const snapshot = engine.snapshot();
        try {
          engine.send({ type: "inc", by: 5 });
        } catch (error) {
          failure = validationFailure(error);
        }
        return { failure, calls, unchanged: stored === before, snapshot };
      } catch (error) {
        return {
          failure: validationFailure(error),
          calls,
          unchanged: stored === before,
        };
      } finally {
        engine?.dispose();
      }
    };
    return {
      invalidLoaded: [createRustValidation, createReferenceValidation].map(
        (create) => run(create, 0, -1),
      ),
      invalidDefaults: [createRustValidation, createReferenceValidation].map(
        (create) => run(create, -1, 7),
      ),
      invalidTransition: [createRustValidation, createReferenceValidation].map(
        (create) => run(create, 0, 7),
      ),
    };
  });
  expect(result.invalidLoaded[0].failure).toEqual(
    result.invalidLoaded[1].failure,
  );
  expect(result.invalidLoaded.every((entry: any) => entry.unchanged)).toBe(
    true,
  );
  expect(result.invalidTransition[0].failure).toEqual(
    result.invalidTransition[1].failure,
  );
  expect(result.invalidTransition.every((entry: any) => entry.unchanged)).toBe(
    true,
  );
  expect(result.invalidTransition[0].calls).toEqual(["read"]);
  expect(result.invalidDefaults[0]).toMatchObject({
    failure: { reason: "invalidContext" },
    calls: [],
    unchanged: true,
  });
  expect(result.invalidDefaults[1].snapshot.context.count).toBe(7);
});
