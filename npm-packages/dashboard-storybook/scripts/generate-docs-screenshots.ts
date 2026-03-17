import * as fs from "node:fs";
import * as path from "node:path";
import { spawn } from "node:child_process";
import { webkit } from "playwright";
import sharp from "sharp";
import pixelmatch from "pixelmatch";
import chalk from "chalk";
import getPort from "get-port";
import ora from "ora";

const SCRIPT_DIR = path.dirname(new URL(import.meta.url).pathname);
const PACKAGE_DIR = path.resolve(SCRIPT_DIR, "..");
const OUTPUT_DIR = path.resolve(
  PACKAGE_DIR,
  "../docs/static/screenshots/storybook",
);
const MANIFEST_PATH = path.resolve(
  PACKAGE_DIR,
  "../docs/src/generated/screenshotManifest.ts",
);

/** Convert PascalCase to snake_case */
function toSnakeCase(str: string): string {
  return str
    .replace(/([A-Z])/g, (m, c, offset) => (offset > 0 ? "_" : "") + c)
    .toLowerCase();
}

/** Derive output filename from story title and theme */
function filenameFromTitle(title: string, theme: "light" | "dark"): string {
  // Strip "docs/" prefix
  const withoutPrefix = title.replace(/^docs\//i, "");
  // Split by "/" and convert each segment to snake_case
  const segments = withoutPrefix.split("/").map(toSnakeCase);
  return `${segments.join("_")}_${theme}.webp`;
}

/** Start the Storybook dev server and wait until it responds, returns { close } */
async function startStorybookDevServer(
  port: number,
): Promise<{ close: () => void }> {
  const proc = spawn(
    "npx",
    ["storybook", "dev", "-p", String(port), "--no-open", "--quiet"],
    { cwd: PACKAGE_DIR, stdio: "ignore" },
  );

  await new Promise<void>((resolve, reject) => {
    proc.on("error", reject);
    proc.on("exit", (code) => {
      if (code !== null && code !== 0) {
        reject(new Error(`Storybook dev server exited with code ${code}`));
      }
    });

    const url = `http://127.0.0.1:${port}/index.json`;
    const poll = async () => {
      try {
        const res = await fetch(url);
        if (res.ok) {
          resolve();
          return;
        }
      } catch {
        /* not ready yet */
      }
      setTimeout(poll, 500);
    };
    poll();
  });

  return { close: () => proc.kill() };
}

// 1. Find an available port and start the Storybook dev server
const port = await getPort();
let spinner = ora(`Starting Storybook dev server on port ${port}...`).start();
const { close: closeServer } = await startStorybookDevServer(port);
spinner.succeed(`Storybook dev server started on port ${port}`);

// 2. Fetch stories from the running dev server
spinner = ora("Fetching stories...").start();
const indexUrl = `http://127.0.0.1:${port}/index.json`;
const indexRes = await fetch(indexUrl);
if (!indexRes.ok) {
  spinner.fail(`Failed to fetch ${indexUrl}: ${indexRes.status}`);
  closeServer();
  process.exit(1);
}
const index = (await indexRes.json()) as {
  entries: Record<string, { id: string; title: string; type: string }>;
};

const docsStories = Object.values(index.entries).filter(
  (e) => e.title.toLowerCase().startsWith("docs/") && e.type === "story",
);

if (docsStories.length === 0) {
  spinner.warn("No docs/ stories found in storybook index.");
}

spinner.succeed(`Found ${docsStories.length} stories`);

// 3. Launch Playwright (WebKit)
spinner = ora("Launching Playwright...").start();
const browser = await webkit.launch();
const context = await browser.newContext({
  viewport: { width: 1024, height: 700 },
  deviceScaleFactor: 2,
});
spinner.succeed("Playwright launched");

// Ensure output dir exists
fs.mkdirSync(OUTPUT_DIR, { recursive: true });

// Track which filenames are current (to detect stale files)
const currentFilenames = new Set<string>();

const results: {
  filename: string;
  theme: "light" | "dark";
  storyTitle: string;
  status: "created" | "updated" | "unchanged";
}[] = [];

// 4. Screenshot each story in light and dark mode
const totalScreenshots = docsStories.length * 2;
let screenshotCount = 0;

for (const story of docsStories) {
  for (const theme of ["light", "dark"] as const) {
    screenshotCount++;
    const filename = filenameFromTitle(story.title, theme);
    currentFilenames.add(filename);
    const outputPath = path.join(OUTPUT_DIR, filename);

    spinner = ora({
      text: `${filename} (${screenshotCount}/${totalScreenshots})`,
      prefixText: "",
    }).start();

    const url = `http://127.0.0.1:${port}/iframe.html?id=${encodeURIComponent(story.id)}&viewMode=story&globals=theme:${theme}`;

    const page = await context.newPage();
    await page.goto(url, { waitUntil: "networkidle" });
    await page.evaluate(() => document.fonts.ready);

    const isComponentStory = story.title
      .toLowerCase()
      .startsWith("docs/components/");

    let png: Buffer;
    let bgColor: string | undefined;
    if (isComponentStory) {
      const root = page.locator(".sb-main-padded");
      bgColor = await page.evaluate(
        () => getComputedStyle(document.body).backgroundColor,
      );
      png = await root.screenshot({ omitBackground: true });
    } else {
      png = await page.screenshot({ fullPage: false });
    }
    await page.close();

    const PADDING = 32;
    const pipeline = sharp(png);
    if (isComponentStory && bgColor) {
      pipeline.trim().extend({
        top: PADDING,
        bottom: PADDING,
        left: PADDING,
        right: PADDING,
        background: bgColor,
      });
    }
    const webp = await pipeline.webp({ lossless: true }).toBuffer();

    let status: "created" | "updated" | "unchanged" = "created";
    if (fs.existsSync(outputPath)) {
      const existing = fs.readFileSync(outputPath);
      const [a, b] = await Promise.all([
        sharp(existing)
          .ensureAlpha()
          .raw()
          .toBuffer({ resolveWithObject: true }),
        sharp(webp).ensureAlpha().raw().toBuffer({ resolveWithObject: true }),
      ]);
      if (a.info.width !== b.info.width || a.info.height !== b.info.height) {
        status = "updated";
      } else {
        const diff = pixelmatch(
          a.data,
          b.data,
          null,
          a.info.width,
          a.info.height,
          { threshold: 0.1 },
        );
        status = diff === 0 ? "unchanged" : "updated";
      }
    }

    if (status !== "unchanged") {
      fs.writeFileSync(outputPath, webp);
    }

    const label = `${filename} (${screenshotCount}/${totalScreenshots})`;
    if (status === "created") {
      spinner.succeed(chalk.green(`created: ${label}`));
    } else if (status === "updated") {
      spinner.warn(chalk.yellow(`updated: ${label}`));
    } else {
      spinner.info(chalk.gray(`unchanged: ${label}`));
    }

    results.push({ filename, theme, storyTitle: story.title, status });
  }
}

// 5. Delete stale screenshots
const deleted: string[] = [];
spinner = ora("Cleaning up stale screenshots...").start();
const existingWebps = fs.existsSync(OUTPUT_DIR)
  ? fs.readdirSync(OUTPUT_DIR).filter((f) => f.endsWith(".webp"))
  : [];
for (const file of existingWebps) {
  if (!currentFilenames.has(file)) {
    fs.unlinkSync(path.join(OUTPUT_DIR, file));
    deleted.push(file);
    console.log(chalk.red(`deleted: ${file}`));
  }
}
if (deleted.length > 0) {
  spinner.succeed(`Deleted ${deleted.length} stale screenshot(s)`);
} else {
  spinner.succeed("No stale screenshots to delete");
}

// 6. Write manifest
spinner = ora("Writing manifest...").start();
const byStory = new Map<string, { light?: string; dark?: string }>();
for (const { filename, theme, storyTitle } of results) {
  if (!byStory.has(storyTitle)) byStory.set(storyTitle, {});
  byStory.get(storyTitle)![theme] = filename;
}

const getDimensions = async (filename: string) => {
  const { width, height } = await sharp(
    path.join(OUTPUT_DIR, filename),
  ).metadata();
  return { width: width!, height: height! };
};

const manifestArray = await Promise.all(
  [...byStory.entries()].map(async ([storyTitle, themes]) => ({
    storyTitle,
    light: themes.light
      ? { filename: themes.light, ...(await getDimensions(themes.light)) }
      : undefined,
    dark: themes.dark
      ? { filename: themes.dark, ...(await getDimensions(themes.dark)) }
      : undefined,
  })),
);

const manifestContent = `// @generated by dashboard-storybook/scripts/generate-docs-screenshots.ts
// Do not edit manually.

export const screenshots = ${JSON.stringify(manifestArray, null, 2)} as const;
`;

fs.mkdirSync(path.dirname(MANIFEST_PATH), { recursive: true });
fs.writeFileSync(MANIFEST_PATH, manifestContent);
spinner.succeed("Manifest written");

// 7. Cleanup
spinner = ora("Cleaning up...").start();
await browser.close();
closeServer();
spinner.succeed("Done!");
