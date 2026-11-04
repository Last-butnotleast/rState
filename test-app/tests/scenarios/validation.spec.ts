import { expect, test } from "@playwright/test";

test("schema validation matches pinned upstream checks and rejects atomically", async ({
  page,
}) => {
  await page.goto("/");
  await expect(
    page.getByRole("button", { name: "Increment", exact: true }),
  ).toBeEnabled();
  const result = await page.evaluate(async () => {
    const paths = {
      rust: "/src/engines/rust-validation.ts",
      reference: "/src/engines/reference-validation.ts",
      runner: "/src/traces/validation.ts",
      scenario: "/src/scenarios/validation.ts",
    };
    const [
      { createRustValidation },
      { createReferenceValidation },
      { runValidation },
      { validationScenario },
    ] = await Promise.all([
      import(paths.rust),
      import(paths.reference),
      import(paths.runner),
      import(paths.scenario),
    ]);
    return [createRustValidation, createReferenceValidation].map((create) =>
      runValidation(validationScenario, create),
    );
  });
  expect(result[0]).toEqual(result[1]);
  const errors = result[0].filter((entry: any) => entry.kind === "error");
  expect(errors.map((entry: any) => entry.reason ?? entry.message)).toEqual([
    "invalidEvent",
    "invalidContext",
    "invalidContext",
    "invalidEmitted",
    "invalidEvent",
    "invalidEvent",
    "unknownEvent",
    "reducer failed",
    "reducer failed",
  ]);
  expect(result[0].filter((entry: any) => entry.kind === "emit")).toEqual([
    { kind: "emit", message: { type: "receipt", total: 2 } },
  ]);
  expect(result[0].at(-1)).toMatchObject({
    kind: "return",
    op: "snapshot",
    result: { context: { count: 7, items: [2], total: 2 } },
  });
});
