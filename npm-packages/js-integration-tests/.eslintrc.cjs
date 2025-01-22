module.exports = {
  env: { browser: true, es2020: true },
  extends: ["eslint:recommended"],
  parserOptions: {
    ecmaVersion: "latest",
    sourceType: "module",
    project: "./tsconfig.json",
    tsconfigRootDir: __dirname,
  },
  settings: { react: { version: "18.2" } },
  plugins: [],
  rules: {
    "@typescript-eslint/no-floating-promises": "error",
  },
};
