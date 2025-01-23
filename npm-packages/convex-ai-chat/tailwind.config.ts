import type { Config } from "tailwindcss";

export default {
  content: ["./lib/**/*.tsx"],
  corePlugins: {
    preflight: false,
  },
  theme: {
    extend: {},
  },
  plugins: [],
  experimental: {
    optimizeUniversalDefaults: true,
  },
} satisfies Config;
