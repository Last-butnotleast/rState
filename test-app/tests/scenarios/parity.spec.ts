import { expect, test } from "@playwright/test";

for (const scenario of ["counter", "composition", "ordering"]) {
  test(`${scenario} matches pinned XState values, identity and ordered traces`, async ({
    page,
  }) => {
    await page.goto("/");
    await expect(
      page.getByRole("button", { name: "Increment", exact: true }),
    ).toBeEnabled();
    const result = await page.evaluate(async (name) => {
      const comparePath = "/src/traces/compare.ts";
      const catalogPath = "/src/scenarios/catalog.ts";
      const { compareScenario } = await import(comparePath);
      const { scenarios } = await import(catalogPath);
      return compareScenario(
        scenarios.find(
          (value: { scenario: string }) => value.scenario === name,
        ),
      );
    }, scenario);
    expect(result.differences, JSON.stringify(result, null, 2)).toEqual([]);
    expect(result.rust.length).toBeGreaterThan(10);
  });
}

test("subscriber reentry and recoverable callback errors match through the WASM bridge", async ({
  page,
}) => {
  await page.goto("/");
  await expect(
    page.getByRole("button", { name: "Increment", exact: true }),
  ).toBeEnabled();
  const results = await page.evaluate(async () => {
    const path = "/src/traces/compare.ts";
    const { compareScenario } = await import(path);
    return [
      compareScenario({
        name: "Reentrant subscriber",
        scenario: "counter",
        steps: [
          {
            op: "subscribe",
            listener: "nested",
            send: { type: "inc", by: 10 },
            whenCount: 1,
          },
          { op: "subscribe", listener: "tail" },
          { op: "send", event: { type: "inc", by: 1 } },
          { op: "snapshot" },
        ],
      }),
      compareScenario({
        name: "Subscriber error",
        scenario: "counter",
        steps: [
          { op: "subscribe", listener: "bad", fail: "subscriber failed" },
          { op: "subscribe", listener: "tail" },
          { op: "send", event: { type: "inc", by: 1 } },
          { op: "snapshot" },
        ],
      }),
      compareScenario({
        name: "Reducer/effect errors",
        scenario: "ordering",
        steps: [
          { op: "subscribe", listener: "a" },
          { op: "send", event: { type: "fail" } },
          { op: "send", event: { type: "effectFail" } },
          { op: "snapshot" },
        ],
      }),
    ];
  });
  for (const result of results)
    expect(result.differences, JSON.stringify(result, null, 2)).toEqual([]);
});
