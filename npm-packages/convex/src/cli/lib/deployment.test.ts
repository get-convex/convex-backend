import { test, expect, vi, beforeEach, afterEach } from "vitest";
import { changesToEnvVarFile, changesToGitIgnore, writeDeploymentEnvVar } from "./deployment.js";
import { Context } from "../../bundler/context.js";

vi.mock("./envvars.js", async (importOriginal) => {
  const actual = (await importOriginal()) as any;
  return {
    ...actual,
    gitIgnoreEnvVarFile: vi.fn().mockResolvedValue(false),
  };
});

const DEPLOYMENT = {
  team: "snoops",
  project: "earth",
  deploymentName: "tall-bar",
};

test("env var changes", () => {
  expect(changesToEnvVarFile(null, "prod", DEPLOYMENT)).toEqual(
    "# Deployment used by `npx convex dev`\n" +
      "CONVEX_DEPLOYMENT=prod:tall-bar # team: snoops, project: earth\n",
  );

  expect(changesToEnvVarFile("CONVEX_DEPLOYMENT=", "prod", DEPLOYMENT)).toEqual(
    "CONVEX_DEPLOYMENT=prod:tall-bar # team: snoops, project: earth",
  );

  expect(
    changesToEnvVarFile("CONVEX_DEPLOYMENT=foo", "prod", DEPLOYMENT),
  ).toEqual("CONVEX_DEPLOYMENT=prod:tall-bar # team: snoops, project: earth");

  expect(changesToEnvVarFile("RAD_DEPLOYMENT=foo", "prod", DEPLOYMENT)).toEqual(
    "RAD_DEPLOYMENT=foo\n" +
      "\n" +
      "# Deployment used by `npx convex dev`\n" +
      "CONVEX_DEPLOYMENT=prod:tall-bar # team: snoops, project: earth\n",
  );

  expect(
    changesToEnvVarFile(
      "RAD_DEPLOYMENT=foo\n" + "CONVEX_DEPLOYMENT=foo",
      "prod",
      DEPLOYMENT,
    ),
  ).toEqual(
    "RAD_DEPLOYMENT=foo\n" +
      "CONVEX_DEPLOYMENT=prod:tall-bar # team: snoops, project: earth",
  );

  expect(
    changesToEnvVarFile(
      "CONVEX_DEPLOYMENT=\n" + "RAD_DEPLOYMENT=foo",
      "prod",
      DEPLOYMENT,
    ),
  ).toEqual(
    "CONVEX_DEPLOYMENT=prod:tall-bar # team: snoops, project: earth\n" +
      "RAD_DEPLOYMENT=foo",
  );
});

test("git ignore changes", () => {
  // Handle additions
  expect(changesToGitIgnore(null)).toEqual(".env.local\n");
  expect(changesToGitIgnore("")).toEqual("\n.env.local\n");
  expect(changesToGitIgnore(".env")).toEqual(".env\n.env.local\n");
  expect(changesToGitIgnore("# .env.local")).toEqual(
    "# .env.local\n.env.local\n",
  );

  // Handle existing
  expect(changesToGitIgnore(".env.local")).toEqual(null);
  expect(changesToGitIgnore(".env.*")).toEqual(null);
  expect(changesToGitIgnore(".env*")).toEqual(null);

  expect(changesToGitIgnore(".env*.local")).toEqual(null);
  expect(changesToGitIgnore("*.local")).toEqual(null);
  expect(changesToGitIgnore("# convex env\n.env.local")).toEqual(null);

  // Handle Windows
  expect(changesToGitIgnore(".env.local\r")).toEqual(null);
  expect(changesToGitIgnore("foo\r\n.env.local")).toEqual(null);
  expect(changesToGitIgnore("foo\r\n.env.local\r\n")).toEqual(null);
  expect(changesToGitIgnore("foo\r\n.env.local\r\nbar")).toEqual(null);

  // Handle trailing whitespace
  expect(changesToGitIgnore(" .env.local ")).toEqual(null);

  // Add .env.local (even if it's negated) to guide the user to solve the problem
  expect(changesToGitIgnore("!.env.local")).toEqual(
    "!.env.local\n.env.local\n",
  );
});

const mockContext = {
  fs: {
    exists: vi.fn(),
    readUtf8File: vi.fn(),
    writeUtf8File: vi.fn(),
  },
} as unknown as Context;

const originalProcessEnv = process.env;

beforeEach(() => {
  vi.clearAllMocks();
  process.env = { ...originalProcessEnv };
});

afterEach(() => {
  process.env = originalProcessEnv;
});

test("writeDeploymentEnvVar skips file creation when CONVEX_DEPLOYMENT exists with correct value", async () => {
  process.env.CONVEX_DEPLOYMENT = "prod:tall-bar";

  const result = await writeDeploymentEnvVar(
    mockContext,
    "prod",
    DEPLOYMENT,
    null,
  );

  expect(result).toEqual({
    wroteToGitIgnore: false,
    changedDeploymentEnvVar: false,
  });

  expect(mockContext.fs.writeUtf8File).not.toHaveBeenCalled();
});
