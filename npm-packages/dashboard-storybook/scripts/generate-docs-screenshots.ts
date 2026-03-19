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

/** Run tasks with limited concurrency */
async function runWithConcurrency<T>(
  tasks: (() => Promise<T>)[],
  concurrency: number,
): Promise<T[]> {
  const results: T[] = [];
  let index = 0;
  const workers = Array.from({ length: concurrency }, async () => {
    while (index < tasks.length) {
      const i = index++;
      results[i] = await tasks[i]();
    }
  });
  await Promise.all(workers);
  return results;
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
const CONCURRENCY = 5;
const total = docsStories.length * 2;
let completed = 0;
const inProgress = new Set<string>();
const errors: { filename: string; error: unknown }[] = [];

function updateSpinner() {
  const lines = [`Screenshots: ${completed}/${total}`];
  for (const f of inProgress) {
    lines.push(chalk.dim(`  ◌ ${f}`));
  }
  spinner.text = lines.join("\n");
}

async function captureScreenshot(
  story: { id: string; title: string; type: string },
  theme: "light" | "dark",
): Promise<{
  filename: string;
  theme: "light" | "dark";
  storyTitle: string;
  status: "created" | "updated" | "unchanged";
} | null> {
  const filename = filenameFromTitle(story.title, theme);
  const outputPath = path.join(OUTPUT_DIR, filename);
  const url = `http://127.0.0.1:${port}/iframe.html?id=${encodeURIComponent(story.id)}&viewMode=story&globals=theme:${theme}`;

  // Read the existing file before any async work so the snapshot is consistent
  // regardless of what other concurrent tasks write during page navigation.
  const existingWebp = fs.existsSync(outputPath)
    ? fs.readFileSync(outputPath)
    : null;

  inProgress.add(filename);
  updateSpinner();

  let context: Awaited<ReturnType<typeof browser.newContext>> | null = null;
  try {
    // Create a fresh browser context for each screenshot to avoid flaky
    // rendering caused by shared state between stories.
    context = await browser.newContext({
      viewport: { width: 1024, height: 700 },
      deviceScaleFactor: 2,
    });
    const page = await context.newPage();
    await page.goto(url, { waitUntil: "networkidle", timeout: 60_000 });
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
    if (existingWebp !== null) {
      if (existingWebp.equals(webp)) {
        // Fast path: identical bytes means identical image.
        status = "unchanged";
      } else {
        // Bytes differ — decode both and do a perceptual comparison to
        // distinguish real changes from minor rendering non-determinism
        // (sub-pixel anti-aliasing, font hinting, etc.).
        const [a, b] = await Promise.all([
          sharp(existingWebp)
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
          // Allow up to 0.05% of pixels to differ — handles minor rendering
          // variations that are invisible to the human eye.
          const maxDiff = Math.ceil(a.info.width * a.info.height * 0.0005);
          status = diff <= maxDiff ? "unchanged" : "updated";
        }
      }
    }

    if (status !== "unchanged") {
      fs.writeFileSync(outputPath, webp);
    }

    inProgress.delete(filename);
    completed++;
    spinner.clear();
    if (status === "created") {
      process.stdout.write(
        chalk.green(`  ✓ ${chalk.white.bgGreen("  Created  ")} ${filename}\n`),
      );
    } else if (status === "updated") {
      process.stdout.write(
        chalk.blue(`  ✓ ${chalk.white.bgBlue("  Updated  ")} ${filename}\n`),
      );
    } else {
      process.stdout.write(
        chalk.gray(`  ✓ ${chalk.white.bgGray(" Unchanged ")} ${filename}\n`),
      );
    }
    updateSpinner();
    spinner.render();

    return { filename, theme, storyTitle: story.title, status };
  } catch (error) {
    inProgress.delete(filename);
    completed++;
    spinner.clear();
    process.stdout.write(chalk.red(`  ✗ failed: ${filename}: ${error}\n`));
    updateSpinner();
    spinner.render();
    errors.push({ filename, error });
    // If the file already existed, preserve it by returning an "unchanged"
    // result so it won't be deleted as stale and stays in the manifest.
    if (existingWebp !== null) {
      return { filename, theme, storyTitle: story.title, status: "unchanged" };
    }
    return null;
  } finally {
    if (context) {
      try {
        await context.close();
      } catch (closeError) {
        process.stdout.write(
          chalk.red(
            `  ✗ failed to close context: ${filename}: ${closeError}\n`,
          ),
        );
      }
    }
  }
}

const tasks = docsStories.flatMap((story) =>
  (["light", "dark"] as const).map(
    (theme) => () => captureScreenshot(story, theme),
  ),
);

spinner = ora(`Screenshots: 0/${total}`).start();

const taskResults = await runWithConcurrency(tasks, CONCURRENCY);

spinner.succeed(`Completed ${total} screenshots`);

for (const result of taskResults) {
  if (result === null) continue;
  currentFilenames.add(result.filename);
  results.push(result);
}

if (errors.length > 0) {
  console.error(chalk.red(`\n${errors.length} screenshot(s) failed:`));
  for (const { filename, error } of errors) {
    console.error(chalk.red(`  ${filename}: ${error}`));
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
    console.log(chalk.red(`  ✓ ${chalk.white.bgRed("  Deleted  ")} ${file}`));
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
