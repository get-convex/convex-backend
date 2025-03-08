/** @type {import('jest').Config} */
module.exports = {
  preset: "ts-jest",
  testEnvironment: "node",
  testMatch: ["**/*.test.ts"],
  transform: {
    "^.+\\.tsx?$": [
      "ts-jest",
      {
        tsconfig: "tsconfig.json",
      },
    ],
  },
  moduleNameMapper: {
    "convex/(.*)": "<rootDir>/__mocks__/convex/$1",
    "^id-encoding$": "<rootDir>/__mocks__/id-encoding",
  },
  moduleDirectories: ["node_modules", "../common/temp/node_modules/.pnpm"],
};
