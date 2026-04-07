/** @type {import('ts-jest/dist/types').InitialOptionsTsJest, import('jest').Config} */

const { join } = require("path");

module.exports = {
  preset: "ts-jest/presets/js-with-ts",
  testEnvironment: "node",
  testTimeout: 10000,
  globals: {
    // jsdom + Jest don't implement fetch which we need to test ConvexHttpClient
    fetch: global.fetch,
  },
  setupFilesAfterEnv: [join(__dirname, "setup-jest.js")],

  // Only run one suite at a time because all of our tests are running against
  // the same backend and we don't want to leak state.
  maxWorkers: 1,
};
