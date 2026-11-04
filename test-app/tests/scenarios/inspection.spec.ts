import { expect, test } from "@playwright/test";

test("inspection matches pinned outer event records and callback-time snapshots", async ({
  page,
}) => {
  await page.goto("/");
  await expect(
    page.getByRole("button", { name: "Increment", exact: true }),
  ).toBeEnabled();
  const result = await page.evaluate(async () => {
    const comparePath = "/src/traces/compare.ts";
    const scenarioPath = "/src/scenarios/inspection.ts";
    const { compareScenario } = await import(comparePath);
    const { inspectionScenario } = await import(scenarioPath);
    return compareScenario(inspectionScenario);
  });
  expect(result.differences, JSON.stringify(result, null, 2)).toEqual([]);
  const records = result.rust.filter(
    (entry: any) => entry.kind === "inspect",
  ) as any[];
  expect(records.length).toBeGreaterThan(10);
  expect(records[0]).toMatchObject({
    event: { type: "@xstate.init" },
    snapshot: { context: { count: 3 } },
  });
  expect(
    records
      .filter((entry) => entry.event.type === "start")
      .map((entry) => entry.snapshot.context.count),
  ).toEqual([4, 4]);
});

test("subscriber-triggered inspection preserves outer records after nested dispatch", async ({
  page,
}) => {
  await page.goto("/");
  await expect(
    page.getByRole("button", { name: "Increment", exact: true }),
  ).toBeEnabled();
  const result = await page.evaluate(async () => {
    const comparePath = "/src/traces/compare.ts";
    const { compareScenario } = await import(comparePath);
    return compareScenario({
      name: "Subscriber inspection reentry",
      scenario: "counter",
      steps: [
        { op: "inspect", listener: "inspection" },
        {
          op: "subscribe",
          listener: "nested",
          whenCount: 1,
          send: { type: "inc", by: 10 },
        },
        { op: "send", event: { type: "inc", by: 1 } },
      ],
    });
  });
  expect(result.differences, JSON.stringify(result, null, 2)).toEqual([]);
  const records = result.rust.filter(
    (entry: any) => entry.kind === "inspect",
  ) as any[];
  expect(
    records.map((entry) => [
      entry.snapshot.context.count,
      entry.current.context.count,
    ]),
  ).toEqual([
    [0, 0],
    [11, 11],
    [1, 11],
  ]);
});
