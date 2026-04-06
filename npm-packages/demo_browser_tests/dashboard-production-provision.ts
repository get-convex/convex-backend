import puppeteer from "puppeteer";
import { loginToDashboard } from "./dashboardHelpers.js";

const main = async () => {
  const browser = await puppeteer.launch({ headless: true });
  const page = await browser.newPage();
  await loginToDashboard(page);

  // Open the dev deployment page (it's nice to test the dev
  // deployment page loads)
  await page.locator("a::-p-text(Development)").click();

  // Open the deployment selector
  await page.locator("#select-deployment").click();

  // Open the production page
  await page.locator("a::-p-text(Production)").click();

  // Make sure we're actually provisioning
  await page.waitForSelector("::-p-text(Provisioning your)");

  // See the Health page
  await page.waitForSelector("::-p-text(Health)");

  await page.close();
  await browser.close();
};
main();
