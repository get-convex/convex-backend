import { argv } from "node:process";
import { Page } from "puppeteer";

export const DASHBOARD_URL = "http://localhost:6789";

export async function loginToDashboard(page: Page, path: string = "") {
  // We end up building large sections of code here, so increase the default
  // timeouts to reduce flakes on CI.
  page.setDefaultTimeout(60000);
  page.setDefaultNavigationTimeout(60000);

  await page.goto(DASHBOARD_URL + path, { waitUntil: "networkidle0" });

  await page.waitForSelector('input[name="email"]', { visible: true });
  await page.type(`input[name="email"]`, argv[2]);

  await Promise.all([
    page.click("aria/Continue"),
    page.waitForNavigation({ waitUntil: "networkidle0" }),
  ]);

  await page.waitForSelector('input[name="password"]', { visible: true });
  await page.type(`input[name="password"]`, argv[3]);

  await Promise.all([
    page.click('button[type="submit"]'),
    page.waitForNavigation(),
  ]);
}
