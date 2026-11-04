import { expect, test } from "@playwright/test";

test("Rust selectors suppress equal projections and emissions deliver filtered listeners first", async ({
  page,
}) => {
  await page.goto("/");
  await expect(
    page.getByRole("button", { name: "Increment", exact: true }),
  ).toBeEnabled();
  const results = await page.evaluate(async () => {
    const rustPath = "/src/engines/rust.ts";
    const referencePath = "/src/engines/reference.ts";
    const { createRust } = await import(rustPath);
    const { createReference } = await import(referencePath);
    return [createRust, createReference].map((create) => {
      const engine = create("composition", () => {});
      const trace: unknown[] = [];
      engine.onAny(() => trace.push("wildcard"));
      engine.onReceipt(() => trace.push("filtered"));
      const stopCount = engine.subscribeSelected("count", (value: number) =>
        trace.push({ count: value }),
      );
      const stopTotal = engine.subscribeSelected("total", (value: number) =>
        trace.push({ total: value }),
      );
      engine.send({ type: "add", price: 8 });
      engine.send({ type: "equal" });
      engine.send({ type: "inc", by: 1 });
      stopCount();
      stopTotal();
      engine.send({ type: "inc", by: 1 });
      engine.dispose();
      return trace;
    });
  });
  expect(results[0]).toEqual(results[1]);
  expect(results[0]).toEqual([
    { total: 8 },
    "filtered",
    "wildcard",
    { count: 1 },
  ]);
});
