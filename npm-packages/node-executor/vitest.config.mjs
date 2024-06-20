import { defineConfig } from "vitest/config";

export default defineConfig({
  test: {
    isolate: true,
    watch: false,
  },
});
