import { defineConfig } from "vitest/config";

export default defineConfig({
  test: {
    typecheck: {
      include: ["type.test.ts"],
      enabled: true,
      tsconfig: "./convex/tsconfig.json",
    },
  },
});
