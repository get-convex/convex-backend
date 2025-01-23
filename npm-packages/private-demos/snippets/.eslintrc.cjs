module.exports = {
  // means don't look to parent dir, so use `root: true` in descendant directories to ignore this config
  // See https://eslint.org/docs/user-guide/configuring/configuration-files#cascading-and-hierarchy
  root: true,
  parser: "@typescript-eslint/parser",
  plugins: ["@typescript-eslint", "react-hooks", "react", "jest"],
  extends: [
    "eslint:recommended",
    "plugin:@typescript-eslint/recommended",
    "prettier",
    "plugin:jest/recommended",
  ],
  env: {
    amd: true,
    browser: true,
    jest: true,
    node: true,
  },
  rules: {
    // any is terrible but we use it a lot (even in our public code).
    "@typescript-eslint/no-explicit-any": "off",

    // asserting that values aren't null is risky but useful.
    "@typescript-eslint/no-non-null-assertion": "off",

    // Add React hooks rules so we don't misuse them.
    "react-hooks/rules-of-hooks": "error",
    "react-hooks/exhaustive-deps": "warn",
    eqeqeq: ["error", "always"],

    // In uncompiled demos we need to demonstrate `require` syntax
    "@typescript-eslint/no-var-requires": "off",

    "jest/expect-expect": "off",
    "jest/no-conditional-expect": "off",

    // This one is different from our standard, because the snippets are partial
    "@typescript-eslint/no-unused-vars": "off",
    // This one is different from our standard, because the snippets are partial
    "@typescript-eslint/no-empty-function": "off",
  },
};
