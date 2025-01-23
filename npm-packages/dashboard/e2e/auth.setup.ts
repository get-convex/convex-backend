// auth.setup.ts
import { expect, test as setup } from "@playwright/test";

const authFile = "playwright/.auth/user.json";

setup("authenticate", async ({ page }) => {
  if (!process.env.E2E_AUTH0_USERNAME) {
    throw new Error("E2E_AUTH0_USERNAME env var is not set");
  }
  if (!process.env.E2E_AUTH0_PASSWORD) {
    throw new Error("E2E_AUTH0_PASSWORD env var is not set");
  }
  if (!process.env.E2E_TEAM_SLUG) {
    throw new Error("E2E_TEAM_SLUG env var is not set");
  }
  await page.goto("/login?allowUsernameAuth=1");
  await expect(page).toHaveURL("/login?allowUsernameAuth=1");
  await page.click("text=Log in with Email");
  await page.fill("input#username", process.env.E2E_AUTH0_USERNAME);
  await page.fill("input#password", process.env.E2E_AUTH0_PASSWORD);
  await page.click("button[name='action']");

  await expect(page).toHaveURL(`/t/${process.env.E2E_TEAM_SLUG}`, {
    timeout: 5000,
  });
  await page.context().storageState({ path: authFile });
});
