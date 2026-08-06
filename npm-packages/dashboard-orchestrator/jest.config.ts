import nextJest from "next/jest.js";

const createJestConfig = nextJest({
  dir: "./",
});

const customJestConfig = {
  moduleDirectories: ["node_modules", "src"],
  testEnvironment: "jest-environment-jsdom",
  setupFilesAfterEnv: ["<rootDir>/setupTests.ts"],
  moduleNameMapper: {
    "^dashboard-common/(.*)$": "<rootDir>/../dashboard-common/src/$1",
    "^@common/(.*)$": "<rootDir>/../dashboard-common/src/$1",
    "^@ui/(.*)$": "<rootDir>/../@convex-dev/design-system/src/$1",
    "^lodash-es$": "lodash",
    // Workspace packages aren't always discoverable from
    // `dashboard-common/src` when jest is invoked from
    // `dashboard-orchestrator`. Map the bare specifier to the workspace
    // package's CJS build so Jest's resolver finds it deterministically.
    // Same fix dashboard-self-hosted applies.
    "^id-encoding$": "<rootDir>/../id-encoding",
  },
  roots: ["<rootDir>"],
};

const config = createJestConfig(customJestConfig);

export default config;
