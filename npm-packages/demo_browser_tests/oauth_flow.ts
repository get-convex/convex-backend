import puppeteer from "puppeteer";
import { argv } from "node:process";
import { sleep } from "./common.js";
import { loginToDashboard } from "./dashboardHelpers.js";

const main = async () => {
  // argv needed by loginToDashboard()
  if (argv.length < 4) {
    console.error("Usage: node dist/oauth_flow.js <username> <password>");
    console.error(
      "OAuth URL should be passed via OAUTH_URL environment variable",
    );
    process.exit(1);
  }

  const authUrl = process.env.OAUTH_URL;
  if (!authUrl) {
    console.error("OAUTH_URL environment variable not set");
    process.exit(1);
  }

  console.log(`Starting OAuth flow with URL: ${authUrl}`);

  const browser = await puppeteer.launch({ headless: true });
  const page = await browser.newPage();

  try {
    // We end up building large sections of code here, so increase the default
    // timeouts to reduce flakes on CI.
    page.setDefaultTimeout(60000);
    page.setDefaultNavigationTimeout(60000);

    console.log(`Original OAuth URL: ${authUrl}`);

    // First, log in to the dashboard normally
    await loginToDashboard(page);

    // After successful login, navigate to the OAuth authorization URL
    console.log(`Navigating to OAuth URL: ${authUrl}`);
    let gotoResponse = await page.goto(authUrl);
    let retries = 0;
    while ((!gotoResponse || gotoResponse.status() >= 400) && retries < 2) {
      console.log(
        `OAuth page returned status ${gotoResponse?.status()}, retrying (attempt ${retries + 1})...`,
      );
      await sleep(3000);
      gotoResponse = await page.goto(authUrl);
      retries++;
    }

    console.log(
      `Logged in and navigated to OAuth authorization page: ${page.url()}`,
    );

    // Wait for the OAuth authorization page to load and click authorize
    await page.waitForSelector("button::-p-text(Authorize)");
    console.log("Found Authorize button, clicking...");
    await page.locator("button::-p-text(Authorize)").click();
    console.log("Clicked Authorize button");

    // Wait a moment to see if anything happens
    await sleep(2000);
    console.log(`After clicking, current URL: ${page.url()}`);

    // Wait for redirect to callback URL
    console.log("Waiting for redirect to callback URL...");
    await page.waitForFunction(() =>
      window.location.href.includes("localhost:8080/callback"),
    );
    console.log(`OAuth flow completed! Redirected to: ${page.url()}`);
  } catch (error) {
    console.error("OAuth flow failed:", error);
    throw error;
  } finally {
    await page.close();
    await browser.close();
  }
};

main().catch((error) => {
  console.error("OAuth flow script failed:", error);
  process.exit(1);
});
