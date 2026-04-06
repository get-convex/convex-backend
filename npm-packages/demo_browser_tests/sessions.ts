import puppeteer from "puppeteer";
import { assertDivWithContent } from "./common.js";
import { argv } from "node:process";

const main = async () => {
  const browser = await puppeteer.launch({ headless: true });
  const page = await browser.newPage();
  await page.goto(`http://127.0.0.1:${argv[2]}`);
  console.log("navigated to page");
  await page.waitForSelector(".badge > span > input");
  await assertDivWithContent(page, "ul", "");
  console.log("Initial content looks good (no messages)");
  await page.type("form > input[placeholder]", "al pastor rocks");
  await page.click("input[type='submit']");
  await assertDivWithContent(page, "span", `al pastor rocks`);
  console.log("Chat message was reflected in the message list, from us");

  await page.close();
  await browser.close();
};
main();
