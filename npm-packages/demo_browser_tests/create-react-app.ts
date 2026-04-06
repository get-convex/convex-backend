import puppeteer from "puppeteer";
import { assertDivWithContent } from "./common.js";

const main = async () => {
  const browser = await puppeteer.launch({ headless: true });
  const page = await browser.newPage();
  await page.goto("http://localhost:3000");
  page.setDefaultTimeout(5000);
  console.log("navigated to page");
  await assertDivWithContent(page, "div", "Here's the counter:");
  await assertDivWithContent(page, "div", "0");
  await assertDivWithContent(page, "button", "Add One!");
  console.log("Initial content looks good");

  await page.click("button");
  console.log("Clicked button");
  await assertDivWithContent(page, "div", "1");
  console.log("Counter incremented!");
  await page.click("button");
  console.log("Clicked button");
  await assertDivWithContent(page, "div", "2");
  console.log("Counter incremented again. It's all happening.");

  await page.close();
  await browser.close();
};
main();
