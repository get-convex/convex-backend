import nextJest from "next/jest";

const createJestConfig = nextJest({
  dir: "./",
});

const customJestConfig = {
  moduleDirectories: ["node_modules", "src"],
  testPathIgnorePatterns: ["dist"],
  testEnvironment: "jest-environment-jsdom",
  setupFilesAfterEnv: ["<rootDir>/setupTests.ts"],
  moduleNameMapper: {
    "react-dnd": "<rootDir>/__mocks__/fileMock.js",
    "react-dnd-scrolling": "<rootDir>/__mocks__/fileMock.js",
  },
};

const config = createJestConfig(customJestConfig);

// eslint-disable-next-line import/no-default-export
export default config;
