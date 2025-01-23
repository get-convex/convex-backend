import type { Config } from "tailwindcss";
import baseConfig from "./tailwind.config.js";

export default {
  ...baseConfig,
  darkMode: ["class", '[data-theme="dark"]'],
} satisfies Config;
