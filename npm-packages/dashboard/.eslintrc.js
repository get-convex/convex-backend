const path = require("path");

module.exports = {
  root: true,
  extends: [path.resolve(__dirname, "../dashboard-common/.eslintrc.cjs")],
  parserOptions: {
    project: true,
    tsconfigRootDir: __dirname,
  },
  plugins: ["boundaries"],
  settings: {
    tailwindcss: {
      config: "../dashboard-common/tailwind.config.ts",
    },
    "boundaries/elements": [
      {
        type: "hooks",
        pattern: "hooks/*",
      },
      {
        type: "elements",
        pattern: "elements/*",
      },
      {
        type: "lib",
        pattern: "lib/*",
      },
      {
        type: "data",
        pattern: "data/*",
      },
      {
        type: "components",
        pattern: "components/*",
        capture: ["feature"],
      },
      {
        type: "pages",
        pattern: "pages/*",
      },
      {
        type: "layouts",
        pattern: "layouts/*",
      },
    ],
  },
  rules: {
    "boundaries/element-types": [
      2,
      {
        default: "disallow",
        rules: [
          {
            from: "hooks",
            allow: ["hooks"],
          },
          {
            from: "elements",
            allow: ["elements"],
          },
          {
            from: "lib",
            allow: ["lib"],
          },
          {
            from: "pages",
            allow: [
              "components",
              "hooks",
              "lib",
              "elements",
              "layouts",
              "data",
            ],
          },
          {
            from: "layouts",
            allow: ["elements"],
          },
          {
            from: "components",
            allow: [
              ["components", { family: "${from.family}" }],
              "hooks",
              "lib",
              "elements",
              "layouts",
              "data",
            ],
          },
        ],
      },
    ],
  },
  overrides: [
    // Next.js and StoryBook requires default exports
    {
      files: [
        "src/pages/**/*.tsx",
        "src/pages/api/**/*.ts",
        "src/**/*.stories.tsx",
      ],
      rules: {
        "import/no-default-export": "off",
      },
    },
  ],
};
