import { expect, test } from "@playwright/test";

test("fixture event matrix and optional payload defaults agree for capability, transition and send", async ({
  page,
}) => {
  await page.goto("/");
  await expect(
    page.getByRole("button", { name: "Increment", exact: true }),
  ).toBeEnabled();
  const failures = await page.evaluate(async () => {
    const path = "/src/traces/compare.ts";
    const { compareScenario } = await import(path);
    const events = [
      { type: "inc", by: 2 },
      { type: "inc" },
      { type: "bump", by: 2 },
      { type: "bump" },
      { type: "keep" },
      { type: "same" },
      { type: "equal" },
      { type: "add", price: 8 },
      { type: "add" },
      { type: "recalculate" },
      { type: "receipt" },
      { type: "start" },
      { type: "fail" },
      { type: "effectFail" },
      { type: "unknown" },
    ];
    return ["counter", "composition", "ordering"].flatMap((scenario) =>
      events.flatMap((event) => {
        const result = compareScenario({
          name: `${scenario} ${event.type}`,
          scenario,
          steps: [
            { op: "subscribe", listener: "a" },
            { op: "can", event },
            { op: "transition", event },
            { op: "snapshot" },
            { op: "send", event },
            { op: "snapshot" },
          ],
        });
        return result.equal ? [] : [{ scenario, event, ...result }];
      }),
    );
  });
  expect(failures).toEqual([]);
});
