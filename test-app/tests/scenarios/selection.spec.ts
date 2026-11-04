import { expect, test } from "@playwright/test";

test("selection chain traces match values, identity, counts and lifecycle", async ({
  page,
}) => {
  await page.goto("/");
  await expect(
    page.getByRole("button", { name: "Increment left", exact: true }),
  ).toBeEnabled();
  const result = await page.evaluate(async () => {
    const tracePath = "/src/traces/selection.ts";
    const scenarioPath = "/src/scenarios/selection.ts";
    const { compareSelectionScenario } = await import(tracePath);
    const { selectionScenario } = await import(scenarioPath);
    return compareSelectionScenario(selectionScenario);
  });
  expect(result.differences, JSON.stringify(result, null, 2)).toEqual([]);
  expect(
    result.rust.filter((entry: { kind: string }) => entry.kind === "select"),
  ).toMatchObject([
    { listener: "c", target: "chain", value: { value: "bucket:1" } },
    { listener: "a", target: "mapped", value: { value: 1 } },
    { listener: "b", target: "mapped", value: { value: 1 } },
    { listener: "d", target: "chain", value: { value: "bucket:3" } },
  ]);
  expect(
    result.rust.findLast(
      (entry: { kind: string; op: string }) =>
        entry.kind === "return" && entry.op === "stats",
    ).result,
  ).toEqual({ source: 7, mapped: 6, chain: 4 });
  expect(
    result.rust.filter((entry: { kind: string }) => entry.kind === "error"),
  ).toEqual([]);
});
