/** @type {import('ts-jest/dist/types').InitialOptionsTsJest} */
module.exports = {
  preset: "ts-jest/presets/js-with-ts",
  testEnvironment: "node",
  testTimeout: 10000,

  // Only run one suite at a time because all of our tests are running against
  // the same backend and we don't want to leak state.
  maxWorkers: 1,
};
