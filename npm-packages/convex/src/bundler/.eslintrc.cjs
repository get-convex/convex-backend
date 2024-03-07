module.exports = {
  rules: {
    "no-restricted-syntax": [
      "error",
      {
        // Copied from `npm-packages/convex/.eslintrc.cjs` because ESLint doesn't merge
        // rules.

        // From https://github.com/typescript-eslint/typescript-eslint/issues/1391#issuecomment-1124154589
        // Prefer `private` ts keyword to `#private` private methods
        selector:
          ":matches(PropertyDefinition, MethodDefinition) > PrivateIdentifier.key",
        message: "Use `private` instead",
      },
      {
        selector: "ThrowStatement",
        message:
          "Don't use `throw` if this is a developer-facing error message and this code could be called by `npx convex dev`. Instead use `ctx.crash`.",
      },
    ],
  },
};
