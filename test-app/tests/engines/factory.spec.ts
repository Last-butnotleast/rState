import { expect, test } from "@playwright/test";

test("one factory creates independent stores that outlive its handle", async ({
  page,
}) => {
  await page.goto("/");
  await expect(
    page.getByRole("button", { name: "Increment left", exact: true }),
  ).toBeEnabled();
  const results = await page.evaluate(async () => {
    const rustPath = "/src/engines/rust-factory.ts";
    const referencePath = "/src/engines/reference-factory.ts";
    const { createRustFactory } = await import(rustPath);
    const { createReferenceFactory } = await import(referencePath);
    return [createRustFactory, createReferenceFactory].map((create) => {
      const factory = create();
      const left = factory.create({ count: 2 });
      const right = factory.create({ count: 9 });
      factory.dispose();
      const direct: string[] = [];
      const selected: number[] = [];
      const emissions: string[] = [];
      left.subscribe(() => direct.push("left"));
      right.subscribe(() => direct.push("right"));
      const unsubscribe = left.subscribeSelected("count", (value: number) =>
        selected.push(value),
      );
      left.onAny(() => emissions.push("left"));
      right.onAny(() => emissions.push("right"));
      try {
        const leftInitial = left.initialSnapshot();
        const rightInitial = right.initialSnapshot();
        left.send({ type: "inc", by: 3 });
        const rightAfterLeft = right.snapshot().context.count;
        right.send({ type: "add", price: 4 });
        unsubscribe();
        left.send({ type: "inc", by: 1 });
        return {
          initial: [leftInitial.context.count, rightInitial.context.count],
          initialStable:
            left.initialSnapshot().snapshotId === leftInitial.snapshotId,
          separateSnapshots: leftInitial !== rightInitial,
          rightAfterLeft,
          final: [left.snapshot().context, right.snapshot().context],
          direct,
          selected,
          emissions,
        };
      } finally {
        left.dispose();
        right.dispose();
      }
    });
  });
  expect(results[0]).toEqual(results[1]);
  expect(results[0]).toMatchObject({
    initial: [2, 9],
    initialStable: true,
    separateSnapshots: true,
    rightAfterLeft: 9,
    final: [
      { count: 6, total: 0 },
      { count: 9, total: 4 },
    ],
    direct: ["left", "right", "left"],
    selected: [5],
    emissions: ["right"],
  });
});
