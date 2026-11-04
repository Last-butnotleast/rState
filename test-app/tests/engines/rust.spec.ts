import { expect, test } from "@playwright/test";

test("real Rust store returns stable identities, synchronous callbacks and explicit unsubscribe", async ({
  page,
}) => {
  await page.goto("/");
  await expect(
    page.getByRole("button", { name: "Increment", exact: true }),
  ).toBeEnabled();
  const result = await page.evaluate(async () => {
    const path = "/src/engines/rust.ts";
    const { createRust } = await import(path);
    const engine = createRust("counter", () => {});
    const before = engine.snapshot();
    const trace: string[] = [];
    const unsubscribe = engine.subscribe(() => trace.push("callback"));
    engine.send({ type: "inc", by: 1 });
    trace.push("returned");
    const after = engine.snapshot();
    const repeated = engine.snapshot();
    unsubscribe();
    unsubscribe();
    engine.send({ type: "inc", by: 1 });
    const count = engine.snapshot().context.count;
    engine.dispose();
    return {
      trace,
      changed: before !== after,
      stable: repeated === after,
      count,
    };
  });
  expect(result).toEqual({
    trace: ["callback", "returned"],
    changed: true,
    stable: true,
    count: 2,
  });
});
