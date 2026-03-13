import { defineConfig } from "vitest/config";

export default defineConfig({
  test: {
    alias: {
      "@jest/globals": "vitest",
    },
    isolate: true,
    watch: false,
    //environment: "node", // "node" is the default
    server: {
      deps: {
        inline: ["@xterm/headless", "@inquirer/testing"],
      },
    },
  },
});
