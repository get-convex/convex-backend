import { argv } from "node:process";
import { Locator, Page } from "puppeteer";

export const DASHBOARD_URL = "http://localhost:6789";

export async function loginToDashboard(page: Page, path: string = "") {
  // We end up building large sections of code here, so increase the default
  // timeouts to reduce flakes on CI.
  page.setDefaultTimeout(60000);
  page.setDefaultNavigationTimeout(60000);

  await page.goto(DASHBOARD_URL + path, { waitUntil: "networkidle0" });

  await page.waitForSelector('input[name="email"]', { visible: true });
  await page.type(`input[name="email"]`, argv[2]);

  // WorkOS AuthKit labels the email-submit button "Continue with email" when
  // social login providers are enabled and "Continue" when they aren't; accept
  // either. A Locator (unlike page.click) also waits for the button to appear.
  await Promise.all([
    Locator.race([
      page.locator("aria/Continue with email"),
      page.locator("aria/Continue"),
    ]).click(),
    page.waitForNavigation({ waitUntil: "networkidle0" }),
  ]);

  await page.waitForSelector('input[name="password"]', { visible: true });
  await page.type(`input[name="password"]`, argv[3]);

  await Promise.all([
    page.click('button[type="submit"]'),
    page.waitForNavigation(),
  ]);

  // WorkOS AuthKit can interject a passkey-enrollment interstitial after
  // sign-in ("Create a passkey for faster and more secure sign in"); the
  // headless browser can't create passkeys, so dismiss it. It only appears
  // for some sessions (server-side rollout), so poll briefly instead of
  // blocking on it.
  const skipPasskey = await page
    .waitForSelector("button::-p-text(Skip for now)", { timeout: 5000 })
    .catch(() => null);
  if (skipPasskey) {
    await Promise.all([skipPasskey.click(), page.waitForNavigation()]);
  }
}
