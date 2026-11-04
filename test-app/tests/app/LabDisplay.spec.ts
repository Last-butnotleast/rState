import { expect, test } from "@playwright/test";

test("controls update Rust, compare the reference and reset on scenario change", async ({
  page,
}) => {
  const errors: string[] = [];
  page.on("pageerror", (error) => errors.push(error.message));
  await page.goto("/");
  await expect(
    page.getByRole("button", { name: "Increment", exact: true }),
  ).toBeEnabled();
  await page.getByRole("button", { name: "Increment", exact: true }).click();
  await expect(page.getByTestId("metric")).toContainText("1");
  await expect(page.getByTestId("trace")).toContainText("observe");
  await page.getByRole("button", { name: "Run comparison" }).click();
  await expect(page.getByRole("status")).toContainText("matching entries");
  await page
    .getByRole("button", { name: "Cart composition", exact: true })
    .click();
  await page.getByRole("button", { name: "Add item · 8", exact: true }).click();
  await expect(page.getByTestId("metric")).toContainText("8");
  await expect(page.getByTestId("trace")).toContainText("receipt");
  await page.getByRole("button", { name: "Run comparison" }).click();
  await expect(page.getByRole("status")).toContainText("matching entries");
  await page
    .getByRole("button", { name: "Effect ordering", exact: true })
    .click();
  await page
    .getByRole("button", { name: "Run nested dispatch", exact: true })
    .click();
  await expect(page.getByTestId("metric")).toContainText("11");
  expect(errors).toEqual([]);
});
