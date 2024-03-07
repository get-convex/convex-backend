import { test, expect } from "@jest/globals";
import { changesToEnvVarFile, changesToGitIgnore } from "./deployment.js";

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
  expect(changesToGitIgnore(null)).toEqual(".env.local\n");

  expect(changesToGitIgnore("")).toEqual("\n.env.local\n");

  expect(changesToGitIgnore(".env")).toEqual(".env\n.env.local\n");

  expect(changesToGitIgnore(".env.local")).toEqual(null);
  expect(changesToGitIgnore(".env.*")).toEqual(null);
  expect(changesToGitIgnore(".env*")).toEqual(null);

  // This is wonky, but will guide the user to solve the problem
  expect(changesToGitIgnore("!.env.local")).toEqual(
    "!.env.local\n.env.local\n",
  );
});
