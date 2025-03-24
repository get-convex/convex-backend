import nextJest from "next/jest";

const createJestConfig = nextJest({
  dir: "./",
});

const customJestConfig = {
  moduleDirectories: ["node_modules", "src"],
  testEnvironment: "jest-environment-jsdom",
  setupFilesAfterEnv: ["<rootDir>/setupTests.ts"],
  moduleNameMapper: {
    "react-dnd": "<rootDir>/__mocks__/fileMock.js",
    "react-dnd-scrolling": "<rootDir>/__mocks__/fileMock.js",
    "^dashboard-common/(.*)$": "<rootDir>/../dashboard-common/src/$1",
    "^@common/(.*)$": "<rootDir>/../dashboard-common/src/$1",
  },
  // Add roots for both packages to run all tests
  roots: ["<rootDir>", "<rootDir>/../dashboard-common"],
};

const config = createJestConfig(customJestConfig);

// eslint-disable-next-line import/no-default-export
export default config;
