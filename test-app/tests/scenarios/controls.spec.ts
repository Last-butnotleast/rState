import { expect, test } from "@playwright/test";

test("reset and bounded snapshot history match the pinned reference", async ({
  page,
}) => {
  await page.goto("/");
  await expect(
    page.getByRole("button", { name: "Increment", exact: true }),
  ).toBeEnabled();
  const result = await page.evaluate(async () => {
    const comparePath = "/src/traces/controls.ts";
    const scenarioPath = "/src/scenarios/controls.ts";
    const { compareControlScenario } = await import(comparePath);
    const { controlsScenario } = await import(scenarioPath);
    return compareControlScenario(controlsScenario);
  });
  expect(result.differences, JSON.stringify(result, null, 2)).toEqual([]);
  const emitted = result.rust.filter(
    (entry: any) => entry.kind === "emit",
  ) as any[];
  expect(emitted).toEqual([
    { kind: "emit", message: { type: "receipt", total: 8 } },
  ]);
  const states = result.rust.filter(
    (entry: any) => entry.kind === "subscribe",
  ) as any[];
  expect(states.every((entry) => entry.value.history.past <= 3)).toBe(true);
  expect(states.some((entry) => entry.value.history.future > 0)).toBe(true);
});

test("hydration reset history and snapshot storage match the actual persist extension", async ({
  page,
}) => {
  await page.goto("/");
  await expect(
    page.getByRole("button", { name: "Increment", exact: true }),
  ).toBeEnabled();
  const result = await page.evaluate(async () => {
    const comparePath = "/src/traces/controls.ts";
    const scenarioPath = "/src/scenarios/controls.ts";
    const { compareControlScenario } = await import(comparePath);
    const { persistenceScenario } = await import(scenarioPath);
    return compareControlScenario(persistenceScenario);
  });
  expect(result.differences, JSON.stringify(result, null, 2)).toEqual([]);
  const initial = result.rust.find(
    (entry: any) => entry.kind === "return" && entry.op === "initial",
  ) as any;
  expect(initial.result.context.count).toBe(20);
  const writes = result.rust.filter(
    (entry: any) => entry.kind === "storageWrite",
  ) as any[];
  expect(writes).toHaveLength(7);
  expect(
    writes.every(
      (entry) => Object.keys(entry.value).sort().join() === "context,version",
    ),
  ).toBe(true);
  expect(writes.some((entry) => entry.value.context.count === 5)).toBe(true);
});
