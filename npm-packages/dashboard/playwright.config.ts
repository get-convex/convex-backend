import { PlaywrightTestConfig, devices } from "@playwright/test";
import path from "path";

const PORT = process.env.PORT || 3000;

const baseURL =
  process.env.PLAYWRIGHT_TEST_BASE_URL || `http://localhost:${PORT}`;

// Reference: https://playwright.dev/docs/test-configuration
const config: PlaywrightTestConfig = {
  timeout: 10 * 1000,
  testDir: path.join(__dirname, "e2e"),
  retries: 0,
  outputDir: "test-results/",

  use: {
    headless: !!process.env.PLAYWRIGHT_TEST_BASE_URL,
    // Use baseURL so to make navigations relative.
    // More information: https://playwright.dev/docs/api/class-testoptions#test-options-base-url
    baseURL,

    // Retry a test if its failing with enabled tracing. This allows you to analyse the DOM, console logs, network traffic etc.
    // More information: https://playwright.dev/docs/trace-viewer
    trace: "retry-with-trace",

    // All available context options: https://playwright.dev/docs/api/class-browser#browser-new-context
    // contextOptions: {
    //   ignoreHTTPSErrors: true,
    // },
  },

  projects: [
    { name: "setup", testMatch: /.*\.setup\.ts/ },
    {
      name: "Desktop Chrome",
      use: {
        ...devices["Desktop Chrome"],
        storageState: "playwright/.auth/user.json",
      },
      dependencies: ["setup"],
    },
    // maybe turn these on one day.
    // {
    //   name: "Desktop Firefox",
    //   use: {
    //     ...devices["Desktop Firefox"],
    //   },
    // },
    // {
    //   name: "Desktop Safari",
    //   use: {
    //     ...devices["Desktop Safari"],
    //   },
    // },
  ],
};
export default config;
