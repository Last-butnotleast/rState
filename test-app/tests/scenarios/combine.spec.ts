import { expect, test } from "@playwright/test";

for (const mode of ["independent", "diamond", "duplicate"]) {
  test(`combined ${mode} traces match values, identities, counts and lifecycle`, async ({
    page,
  }) => {
    await page.goto("/");
    await expect(
      page.getByRole("button", { name: "Increment left", exact: true }),
    ).toBeEnabled();
    const result = await page.evaluate(async (mode) => {
      const tracePath = "/src/traces/combine.ts";
      const scenarioPath = "/src/scenarios/combine.ts";
      const { compareCombineScenario } = await import(tracePath);
      const { combineScenario } = await import(scenarioPath);
      return compareCombineScenario(combineScenario(mode));
    }, mode);
    expect(result.differences, JSON.stringify(result, null, 2)).toEqual([]);
    expect(
      result.rust.some((entry: { kind: string }) => entry.kind === "select"),
    ).toBe(true);
    expect(
      result.rust.filter((entry: { kind: string }) => entry.kind === "error"),
    ).toEqual([]);
  });
}
