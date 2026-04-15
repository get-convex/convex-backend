import puppeteer, { type Browser, type Page } from "puppeteer";
import * as path from "node:path";
import * as fs from "node:fs";

export async function withBrowser(
  testFn: (page: Page, browser: Browser) => Promise<void>,
): Promise<void> {
  const browser = await puppeteer.launch({ headless: true });
  const page = await browser.newPage();
  try {
    await testFn(page, browser);
  } catch (error) {
    try {
      const screenshotDir = process.env.SCREENSHOT_DIR || ".";
      const testName = path.basename(process.argv[1] || "unknown", ".js");
      const filename = `${testName}-failure-${Date.now()}.png`;
      const filepath = path.join(screenshotDir, filename);
      fs.mkdirSync(screenshotDir, { recursive: true });
      await page.screenshot({ path: filepath, fullPage: true });
      console.error(`Screenshot saved to: ${filepath}`);
    } catch (screenshotError) {
      console.error("Failed to capture screenshot:", screenshotError);
    }
    throw error;
  } finally {
    await page.close();
    await browser.close();
  }
}

export const assertDivWithContent = async (
  page: Page,
  selector: string,
  innerText: string,
) => {
  // The first argument to `page.waitForFunction()` is not a closure so closed
  // over variables need to be passed explicitly.
  await page.waitForFunction(
    (selector: string, innerText: string) => {
      const divs = [...document.querySelectorAll(selector)] as HTMLDivElement[];
      return divs.some((div: HTMLDivElement) => div.innerText === innerText);
    },
    {},
    selector,
    innerText,
  );
};

export const sleep = (durationMs: number) =>
  new Promise((r) => setTimeout(r, durationMs));
