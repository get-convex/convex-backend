const path = require("path");

module.exports = {
  root: true,
  extends: [path.resolve(__dirname, "../dashboard-common/.eslintrc.cjs")],
  parserOptions: {
    project: true,
    tsconfigRootDir: __dirname,
  },
  plugins: [],
  overrides: [
    // Next.js requires default exports for pages and API routes.
    {
      files: ["src/pages/**/*.tsx", "src/pages/api/**/*.ts"],
      rules: {
        "import/no-default-export": "off",
      },
    },
  ],
};
