import { expect, test } from "@playwright/test";

test("two input-created stores match pinned factory traces and instance isolation", async ({
  page,
}) => {
  await page.goto("/");
  await expect(
    page.getByRole("button", { name: "Increment left", exact: true }),
  ).toBeEnabled();
  const result = await page.evaluate(async () => {
    const tracePath = "/src/traces/factory.ts";
    const scenarioPath = "/src/scenarios/factory.ts";
    const { compareFactoryScenario } = await import(tracePath);
    const { factoryScenario } = await import(scenarioPath);
    return compareFactoryScenario(factoryScenario);
  });
  expect(result.differences, JSON.stringify(result, null, 2)).toEqual([]);
  expect(
    result.rust.filter((entry: { kind: string }) => entry.kind === "error"),
  ).toHaveLength(1);
  expect(
    result.rust.filter((entry: { kind: string }) => entry.kind === "select"),
  ).toMatchObject([
    { store: "left", listener: "selected-a", value: 5 },
    { store: "right", listener: "selected-b", value: 4 },
  ]);
  expect(
    result.rust.filter((entry: { kind: string }) => entry.kind === "emit"),
  ).toMatchObject([{ store: "right", message: { type: "receipt", total: 4 } }]);
  for (const [store, count, total] of [
    ["left", 6, 0],
    ["right", 11, 4],
  ]) {
    const last = result.rust.findLast(
      (entry: { kind: string; op: string; store: string }) =>
        entry.kind === "return" &&
        entry.op === "snapshot" &&
        entry.store === store,
    );
    expect(last.result.context).toMatchObject({ count, total });
  }
});
