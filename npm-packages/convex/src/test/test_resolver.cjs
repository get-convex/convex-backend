module.exports = (path, options) => {
  // Call the defaultResolver, so we leverage its cache, error handling, etc.
  return options.defaultResolver(path, {
    ...options,
    // Use packageFilter to process parsed `package.json` before the resolution (see https://www.npmjs.com/package/resolve#resolveid-opts-cb)
    packageFilter: (pkg) => {
      // Force the `ws` import to use CJS in both the jest jsdom browser environment and the
      // jest node environment.
      //
      // jest-environment-jsdom 28+ tries to use browser exports instead of default exports,
      // but since we have a file that is imported from both types of tests (jsdom and node),
      // we need to make sure we're importing the CJS one in both cases.
      //
      // This workaround prevents Jest from considering ws's module-based exports at all;
      // it falls back to ws's CommonJS+node "main" property.
      //
      // Inspired by https://github.com/microsoft/accessibility-insights-web/pull/5421#issuecomment-1109168149
      // This can go away once we improve `client_node_test_helpers.ts` to have different behavior
      // in node vs jsdom. But we can't do this until WS publishes types for both (or unifies their behavior)
      if (pkg.name === "ws") {
        delete pkg["exports"];
        delete pkg["module"];
      }
      return pkg;
    },
  });
};
