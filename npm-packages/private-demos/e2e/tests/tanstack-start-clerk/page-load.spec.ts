import { test, expect } from "@playwright/test";

test("page loads without JavaScript errors", async ({ page }) => {
  const errors: string[] = [];
  page.on("pageerror", (err) => errors.push(err.message));

  await page.goto("/");
  await page.waitForLoadState("networkidle");

  expect(errors).toEqual([]);
});

test("page renders content", async ({ page }) => {
  await page.goto("/");
  await expect(page.getByText("Hello Clerk!")).toBeVisible();
});

test("sign-in button is visible", async ({ page }) => {
  await page.goto("/");
  await expect(page.getByRole("button", { name: /sign in/i })).toBeVisible();
});
