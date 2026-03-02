import { defineConfig } from "vitest/config";

export default defineConfig({
  test: {
    projects: [
      {
        extends: true,
        test: {
          name: "convex",
          include: ["convex/**/*.test.{ts,js}"],
          environment: "edge-runtime",
        },
      },
      {
        extends: true,
        test: {
          name: "frontend",
          include: ["**/*.test.{ts,tsx,js,jsx}"],
          exclude: ["convex/**", "**/node_modules/**"],
          environment: "jsdom",
        },
      },
    ],
  },
});
