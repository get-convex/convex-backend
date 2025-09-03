const path = require("path");

module.exports = {
  root: true,
  extends: [path.resolve(__dirname, "../dashboard-common/.eslintrc.cjs")],
  parserOptions: {
    project: true,
    tsconfigRootDir: __dirname,
  },
  settings: {
    "better-tailwindcss": {
      entryPoint: "./.storybook/preview.css",
    },
  },
};
