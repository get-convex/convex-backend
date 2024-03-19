module.exports = {
  root: true,
  parser: "@typescript-eslint/parser",
  plugins: ["@typescript-eslint", "react-hooks", "react"],
  extends: [
    "eslint:recommended",
    "plugin:@typescript-eslint/recommended",
    "prettier",
  ],
  rules: {
    // we use any to access internal-only APIs in Convex functions
    "@typescript-eslint/no-explicit-any": "off",
    "@typescript-eslint/no-unused-vars": [
      "error",
      { varsIgnorePattern: "_.*" },
    ],
    "@typescript-eslint/no-floating-promises": "error",
    // system UDF argument validation
    "no-restricted-imports": [
      "error",
      {
        patterns: [
          {
            group: ["**/_generated/server"],
            importNames: [
              "query",
              "mutation",
              "action",
              "internalQuery",
              "internalMutation",
              "internalAction",
            ],
            message:
              "Use the query wrappers from convex/server.ts instead for system UDF argument validation.",
          },
          {
            group: ["convex/server"],
            importNames: [
              "queryGeneric",
              "mutationGeneric",
              "actionGeneric",
              "internalQueryGeneric",
              "internalMutationGeneric",
              "internalActionGeneric",
            ],
            message:
              "Use the query wrappers from convex/server.ts instead for system UDF argument validation.",
          },
        ],
      },
    ],
  },
  parserOptions: {
    project: "./tsconfig.json",
    tsconfigRootDir: __dirname,
  },
  ignorePatterns: ["node_modules", "dist", "_generated"],
};
