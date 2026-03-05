import { test, expect, beforeAll, vi, MockInstance } from "vitest";
import { _deploymentCredentialsOrConfigure } from "./configure.js";
import { oneoffContext } from "../bundler/context.js";
import { Context } from "../bundler/context.js";
import { DeploymentSelection } from "./lib/deploymentSelection.js";

let stderrSpy: MockInstance;

beforeAll(async () => {
  stderrSpy = vi.spyOn(process.stderr, "write").mockImplementation(() => true);
});

function makeProdDeployKeySelection(): DeploymentSelection {
  return {
    kind: "existingDeployment",
    deploymentToActOn: {
      url: "https://prod-deployment.convex.cloud",
      adminKey: "prod:tall-forest-1234|fakekey",
      deploymentFields: {
        deploymentName: "tall-forest-1234",
        deploymentType: "prod",
        projectSlug: "my-project",
        teamSlug: "my-team",
      },
      source: "deployKey",
    },
  };
}

function makeDevDeployKeySelection(): DeploymentSelection {
  return {
    kind: "existingDeployment",
    deploymentToActOn: {
      url: "https://dev-deployment.convex.cloud",
      adminKey: "dev:happy-animal-5678|fakekey",
      deploymentFields: {
        deploymentName: "happy-animal-5678",
        deploymentType: "dev",
        projectSlug: "my-project",
        teamSlug: "my-team",
      },
      source: "deployKey",
    },
  };
}

const baseCmdOptions = {
  selectionWithinProject: { kind: "ownDev" as const },
  prod: false,
  localOptions: {
    forceUpgrade: false,
  },
};

test("_deploymentCredentialsOrConfigure rejects prod deploy key without --prod flag", async () => {
  const originalContext = await oneoffContext({
    url: undefined,
    adminKey: undefined,
    envFile: undefined,
  });

  let crashMessage: string | null = null;
  const ctx: Context = {
    ...originalContext,
    crash: (args: { printedMessage: string | null }) => {
      crashMessage = args.printedMessage;
      throw new Error("crash");
    },
  } as unknown as Context;

  const selection = makeProdDeployKeySelection();

  await expect(
    _deploymentCredentialsOrConfigure(ctx, selection, null, baseCmdOptions),
  ).rejects.toThrow("crash");

  expect(crashMessage).toContain(
    "`npx convex dev` cannot be used with a production deployment",
  );
  expect(crashMessage).toContain("CONVEX_DEPLOY_KEY");
});

test("_deploymentCredentialsOrConfigure allows prod deploy key with --prod flag", async () => {
  const originalContext = await oneoffContext({
    url: undefined,
    adminKey: undefined,
    envFile: undefined,
  });

  const ctx: Context = {
    ...originalContext,
    crash: (args: { printedMessage: string | null }) => {
      throw new Error(`Unexpected crash: ${args.printedMessage}`);
    },
  } as unknown as Context;

  const selection = makeProdDeployKeySelection();

  const result = await _deploymentCredentialsOrConfigure(ctx, selection, null, {
    ...baseCmdOptions,
    prod: true,
  });

  expect(result.url).toBe("https://prod-deployment.convex.cloud");
  expect(result.deploymentFields?.deploymentType).toBe("prod");
});

test("_deploymentCredentialsOrConfigure allows dev deploy key without --prod flag", async () => {
  const originalContext = await oneoffContext({
    url: undefined,
    adminKey: undefined,
    envFile: undefined,
  });

  const ctx: Context = {
    ...originalContext,
    crash: (args: { printedMessage: string | null }) => {
      throw new Error(`Unexpected crash: ${args.printedMessage}`);
    },
  } as unknown as Context;

  const selection = makeDevDeployKeySelection();

  const result = await _deploymentCredentialsOrConfigure(
    ctx,
    selection,
    null,
    baseCmdOptions,
  );

  expect(result.url).toBe("https://dev-deployment.convex.cloud");
  expect(result.deploymentFields?.deploymentType).toBe("dev");
});
