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
    "lodash-es": "<rootDir>/../dashboard/node_modules/lodash",
    // Force a single React copy: the convex workspace package still has
    // react@18 in its own node_modules, and mixing it with the dashboard's
    // react@19 makes rendering fail at test time.
    "^react$": "<rootDir>/node_modules/react",
    "^react/(.*)$": "<rootDir>/node_modules/react/$1",
    "^react-dom$": "<rootDir>/node_modules/react-dom",
    "^react-dom/(.*)$": "<rootDir>/node_modules/react-dom/$1",
  },
  roots: [
    "<rootDir>",
    "<rootDir>/../dashboard-common",
    "<rootDir>/../@convex-dev/design-system",
  ],
};

const config = createJestConfig(customJestConfig);

export default config;
