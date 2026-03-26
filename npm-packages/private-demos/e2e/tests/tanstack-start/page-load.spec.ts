import { test, expect } from "@playwright/test";

test("page loads without JavaScript errors", async ({ page }) => {
  const errors: string[] = [];
  page.on("pageerror", (err) => errors.push(err.message));

  await page.goto("/");
  await page.waitForLoadState("networkidle");

  expect(errors).toEqual([]);
});

test("page renders navigation links", async ({ page }) => {
  await page.goto("/");
  await expect(
    page.getByText("Simple sibling queries with no router"),
  ).toBeVisible();
  await expect(
    page.getByText("One component with two useSuspenseQuery calls"),
  ).toBeVisible();
});

test("navigation links work without errors", async ({ page }) => {
  const errors: string[] = [];
  page.on("pageerror", (err) => errors.push(err.message));

  await page.goto("/");
  await page.getByText("Simple sibling queries with no router").click();
  await page.waitForLoadState("networkidle");

  expect(errors).toEqual([]);
});
