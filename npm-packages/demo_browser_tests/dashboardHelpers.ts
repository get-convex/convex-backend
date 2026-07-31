import { argv } from "node:process";
import { Locator, Page } from "puppeteer";
import { noteBrowserEvent } from "./common.js";

export const DASHBOARD_URL = "http://localhost:6789";

// Per-attempt budget for reaching the hosted login form. The first attempt also
// pays for Next.js compiling the route on demand, so it keeps the full timeout;
// retries hit a warm server.
const LOGIN_FORM_ATTEMPT_TIMEOUTS_MS = [60000, 20000, 20000];

// Navigate to the dashboard and wait for AuthKit's hosted login form.
// Retry on failure: AuthKit sometimes paints only its background and never
// renders the form; a fresh navigation clears that blank page.
async function gotoLoginForm(page: Page, path: string) {
  const attempts = LOGIN_FORM_ATTEMPT_TIMEOUTS_MS.length;
  for (const [attempt, timeout] of LOGIN_FORM_ATTEMPT_TIMEOUTS_MS.entries()) {
    try {
      await page.goto(DASHBOARD_URL + path, { waitUntil: "networkidle0" });
      await page.waitForSelector('input[name="email"]', {
        visible: true,
        timeout,
      });
      if (attempt > 0) {
        // The retry salvaged a run that used to fail outright. Counting these is
        // the only way to tell "AuthKit never blanked" from "it blanked and we
        // recovered", so record it where a passing test can't swallow it.
        noteBrowserEvent(
          `login-form-recovered after ${attempt} blank render(s), ` +
            `on attempt ${attempt + 1}/${attempts}`,
        );
      }
      return;
    } catch (error) {
      const message =
        `login form did not render at ${page.url()} ` +
        `(attempt ${attempt + 1}/${attempts}): ${error}`;
      console.error(message);
      noteBrowserEvent(`login-form-blank ${message}`);
      if (attempt === attempts - 1) {
        noteBrowserEvent("login-form-gave-up");
        throw error;
      }
    }
  }
}

export async function loginToDashboard(page: Page, path: string = "") {
  // We end up building large sections of code here, so increase the default
  // timeouts to reduce flakes on CI.
  page.setDefaultTimeout(60000);
  page.setDefaultNavigationTimeout(60000);

  await gotoLoginForm(page, path);
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
