import { expect, test } from "@playwright/test";

test("factory controls update one input-created Rust store at a time", async ({
  page,
}) => {
  const errors: string[] = [];
  page.on("pageerror", (error) => errors.push(error.message));
  await page.goto("/");
  const panel = page.getByRole("region", { name: "Reusable store factory" });
  await expect(panel.getByTestId("factory-left")).toContainText("2");
  await expect(panel.getByTestId("factory-right")).toContainText("9");
  await panel
    .getByRole("button", { name: "Increment left", exact: true })
    .click();
  await expect(panel.getByTestId("factory-left")).toContainText("3");
  await expect(panel.getByTestId("factory-right")).toContainText("9");
  await panel
    .getByRole("button", { name: "Increment right", exact: true })
    .click();
  await expect(panel.getByTestId("factory-left")).toContainText("3");
  await expect(panel.getByTestId("factory-right")).toContainText("10");
  await panel
    .getByRole("button", { name: "Compare factory", exact: true })
    .click();
  await expect(panel.getByTestId("factory-comparison")).toContainText(
    "factory trace entries match",
  );
  expect(errors).toEqual([]);
});
