import nextJest from "next/jest";

const createJestConfig = nextJest({
  dir: "./",
});

const customJestConfig = {
  moduleDirectories: ["node_modules", "src"],
  testEnvironment: "jest-environment-jsdom",
  setupFilesAfterEnv: ["<rootDir>/setupTests.ts"],
};

const config = createJestConfig(customJestConfig);

// eslint-disable-next-line import/no-default-export
export default config;
