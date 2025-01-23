import { test, expect } from "@playwright/test";

test.describe("navigation tests", () => {
  test.beforeEach(async ({ page }) => {
    await page.goto("/");
    // Look for the tutorial project
    await page.getByText("tutorial", { exact: true }).click();
  });

  test("navigates to all deployment tabs", async ({ page }) => {
    // DATA TAB
    await expect(page).toHaveURL(/data\?table=messages$/);

    // Check that the page claims there is 1 document in the table
    await expect(page.getByText("1 document")).toBeInViewport();

    // FUNCTIONS TAB
    await page.getByText("Functions").click();

    await expect(page).toHaveURL(/functions$/);

    // Click on the listMessages function to make sure the page works.
    await page.click("text=listMessages");
    await expect(page).toHaveURL(/functions\?function=listMessages$/);

    // CRONS TAB
    await page.getByText("Cron Jobs").click();

    await expect(page).toHaveURL(/cron-jobs$/);

    await expect(
      page.getByText("Run backend code on a regular schedule"),
    ).toBeInViewport();

    // LOGS TAB
    await page.getByText("Logs").click();

    await expect(page).toHaveURL(/logs$/);
    await expect(
      page.getByText(
        "This page is a realtime stream of events occuring within this deployment.",
      ),
    ).toBeInViewport();

    // HISTORY TAB
    await page.getByText("History").click();

    await expect(page).toHaveURL(/history$/);

    // The only stable text on this page right now is that "Dates" filter, so look for that.
    await expect(page.getByText("Dates")).toBeInViewport();

    // SETTINGS TAB
    await page.getByText("Settings", { exact: true }).click();

    await expect(page).toHaveURL(/settings$/);

    // Check that the page loaded by looking for the Deployment Settings text
    await expect(page.getByText("Deployment Settings")).toBeInViewport();
  });
});
