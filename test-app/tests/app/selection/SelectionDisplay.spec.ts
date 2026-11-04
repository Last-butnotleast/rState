import { expect, test } from "@playwright/test";

test("selection controls expose cached chained values from real Rust", async ({
  page,
}) => {
  const errors: string[] = [];
  page.on("pageerror", (error) => errors.push(error.message));
  await page.goto("/");
  const panel = page.getByRole("region", {
    name: "Selection composition",
    exact: true,
  });
  await expect(panel.getByTestId("selection-chain")).toHaveText("bucket:0");
  await panel
    .getByRole("button", { name: "Increment source", exact: true })
    .click();
  await expect(panel.getByTestId("selection-source")).toHaveText("1");
  await expect(panel.getByTestId("selection-mapped")).toHaveText("0");
  await expect(panel.getByTestId("selection-chain")).toHaveText("bucket:0");
  await panel
    .getByRole("button", { name: "Increment source", exact: true })
    .click();
  await expect(panel.getByTestId("selection-chain")).toHaveText("bucket:1");
  await panel
    .getByRole("button", { name: "Compare selections", exact: true })
    .click();
  await expect(panel.getByTestId("selection-comparison")).toContainText(
    "selection trace entries match",
  );
  expect(errors).toEqual([]);
});
