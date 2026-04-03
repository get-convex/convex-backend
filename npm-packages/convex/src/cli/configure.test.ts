import { describe, it, expect, vi, beforeEach } from "vitest";
import { updateEnvAndConfigForDeploymentSelection } from "./configure.js";
import { writeDeploymentEnvVar } from "./lib/deployment.js";
import { finalizeConfiguration } from "./lib/init.js";

vi.mock("./lib/deployment.js", async (importOriginal) => {
  const actual = await importOriginal<typeof import("./lib/deployment.js")>();
  return {
    ...actual,
    writeDeploymentEnvVar: vi.fn().mockResolvedValue({
      wroteToGitIgnore: false,
      changedDeploymentEnvVar: true,
    }),
  };
});

vi.mock("./lib/config.js", async (importOriginal) => {
  const actual = await importOriginal<typeof import("./lib/config.js")>();
  return {
    ...actual,
    readProjectConfig: vi.fn().mockResolvedValue({
      configPath: "convex.json",
      projectConfig: { functions: "convex" },
    }),
    writeProjectConfig: vi.fn().mockResolvedValue(undefined),
    ensureConvexFunctionsDir: vi.fn().mockResolvedValue(undefined),
  };
});

vi.mock("./lib/init.js", async (importOriginal) => {
  const actual = await importOriginal<typeof import("./lib/init.js")>();
  return {
    ...actual,
    finalizeConfiguration: vi.fn().mockResolvedValue(undefined),
  };
});

const ctx = {} as any;

describe("updateEnvAndConfigForDeploymentSelection", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("prod deployments update CONVEX_URL and CONVEX_SITE_URL but not CONVEX_DEPLOYMENT", async () => {
    await updateEnvAndConfigForDeploymentSelection(
      ctx,
      {
        url: "https://happy-animal-123.convex.cloud",
        siteUrl: "https://happy-animal-123.convex.site",
        deploymentName: "happy-animal-123",
        teamSlug: "my-team",
        projectSlug: "my-project",
        deploymentType: "prod",
      },
      null,
    );

    expect(writeDeploymentEnvVar).not.toHaveBeenCalled();
    expect(finalizeConfiguration).toHaveBeenCalledWith(
      ctx,
      expect.objectContaining({
        changedDeploymentEnvVar: false,
        wroteToGitIgnore: false,
        url: "https://happy-animal-123.convex.cloud",
        siteUrl: "https://happy-animal-123.convex.site",
      }),
    );
  });

  it("dev deployments update CONVEX_DEPLOYMENT, CONVEX_URL, and CONVEX_SITE_URL", async () => {
    await updateEnvAndConfigForDeploymentSelection(
      ctx,
      {
        url: "https://joyful-capybara-456.convex.cloud",
        siteUrl: "https://joyful-capybara-456.convex.site",
        deploymentName: "joyful-capybara-456",
        teamSlug: "my-team",
        projectSlug: "my-project",
        deploymentType: "dev",
      },
      null,
    );

    expect(writeDeploymentEnvVar).toHaveBeenCalledWith(
      ctx,
      "dev",
      {
        team: "my-team",
        project: "my-project",
        deploymentName: "joyful-capybara-456",
      },
      null,
    );
    expect(finalizeConfiguration).toHaveBeenCalledWith(
      ctx,
      expect.objectContaining({
        changedDeploymentEnvVar: true,
        url: "https://joyful-capybara-456.convex.cloud",
        siteUrl: "https://joyful-capybara-456.convex.site",
      }),
    );
  });

  it("local deployments update CONVEX_DEPLOYMENT, CONVEX_URL, and CONVEX_SITE_URL", async () => {
    await updateEnvAndConfigForDeploymentSelection(
      ctx,
      {
        url: "https://swift-squirrel-789.convex.cloud",
        siteUrl: "https://swift-squirrel-789.convex.site",
        deploymentName: "swift-squirrel-789",
        teamSlug: "my-team",
        projectSlug: "my-project",
        deploymentType: "local",
      },
      "existing-deployment-name",
    );

    expect(writeDeploymentEnvVar).toHaveBeenCalledWith(
      ctx,
      "local",
      {
        team: "my-team",
        project: "my-project",
        deploymentName: "swift-squirrel-789",
      },
      "existing-deployment-name",
    );
    expect(finalizeConfiguration).toHaveBeenCalledWith(
      ctx,
      expect.objectContaining({
        changedDeploymentEnvVar: true,
        url: "https://swift-squirrel-789.convex.cloud",
        siteUrl: "https://swift-squirrel-789.convex.site",
      }),
    );
  });
});
