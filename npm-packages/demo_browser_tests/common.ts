import type { Page } from "puppeteer";

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
