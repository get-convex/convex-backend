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
        type: "components",
        pattern: "components/*",
        capture: ["feature"],
      },
      {
        type: "providers",
        pattern: "providers/*",
      },
      {
        type: "api",
        pattern: "api/*",
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
            from: "providers",
            allow: ["providers"],
          },
          {
            from: "api",
            allow: ["api"],
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
            allow: ["components", "hooks", "lib", "elements", "layouts"],
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
