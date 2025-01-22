import type { Config } from "tailwindcss";

const config: Config = {
  content: ["./src/**/*.{js,jsx,ts,tsx}"],
  theme: {
    colors: {
      neutral: {
        white: "#ffffff",
        n1: "#f6f6f6",
        n2: "#f1f1f1",
        n3: "#e5e5e5",
        n4: "#d7d7d7",
        n5: "#c2c2c2",
        n6: "#a9a9ac",
        n7: "#8b8b8e",
        n8: "#6d6d70",
        n9: "#4f4f52",
        n10: "#38383a",
        n11: "#292929",
        n12: "#141414",
        n13: "#111111",
        black: "#000000",
      },
      plum: {
        p1: "#f4e9f1",
        p2: "#e3d0df",
        p3: "#d7b3cf",
        p4: "#8d2676",
        p5: "#711e5e",
        p6: "#47133b",
      },
      yellow: {
        y1: "#fdefd2",
        y2: "#f8d077",
        y3: "#f3b01c",
        y4: "#e7a71b",
      },
      red: {
        r1: "#fcd6d5",
        r2: "#f15d59",
        r3: "#ee342f",
        r4: "#d62f2a",
      },
      green: {
        g1: "#e5f3dc",
        g2: "#72c043",
        g3: "#4fb014",
        g4: "#479e12",
      },
      transparent: "transparent",
    },
    fontFamily: {
      sans: ["Inter", "sans-serif"],
    },
  },
  plugins: [],
  darkMode: ["class", '[data-theme="dark"]'],
  corePlugins: {
    preflight: false,
  },
};

export default config;
