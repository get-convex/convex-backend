import { defineConfig } from "vitest/config";

export default defineConfig({
  test: {
    // Reference the setup file we created
    setupFiles: ["./vitest.setup.ts"],

    // Optional: set globals to true if you want to avoid using vitest. prefix
    // This is helpful if you're migrating from Jest
    globals: true,

    // Environment configuration
    environment: "node",

    // Include pattern for test files
    include: ["**/*.test.ts", "**/*.spec.ts"],

    // Optional: TypeScript configuration
    typecheck: {
      enabled: true,
      tsconfig: "./tsconfig.json",
    },
  },
});
