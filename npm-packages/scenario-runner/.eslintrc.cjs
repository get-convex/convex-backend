module.exports = {
  env: { browser: true, es2020: true },
  extends: ["eslint:recommended"],
  parserOptions: {
    ecmaVersion: "latest",
    sourceType: "module",
    project: "./tsconfig.json",
    tsconfigRootDir: __dirname,
  },
  ignorePatterns: ["convex/_generated", "convex.config.ts"],
  settings: { react: { version: "18.2" } },
  rules: {
    "require-await": "error",
    "@typescript-eslint/no-floating-promises": "error",
  },
};
