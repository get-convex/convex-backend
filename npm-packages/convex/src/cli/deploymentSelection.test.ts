import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { nodeFs } from "../bundler/fs.js";
import { env } from "./env.js";
import { deploy } from "./deploy.js";
import { dev } from "./dev.js";
import {
  deploymentFetch,
  bigBrainAPI,
  bigBrainAPIMaybeThrows,
} from "./lib/utils/utils.js";
import { readGlobalConfig } from "./lib/utils/globalConfig.js";
import { deployToDeployment } from "./lib/deploy2.js";
import { runPush } from "./lib/components.js";
import { readProjectConfig, getAuthKitConfig } from "./lib/config.js";
import { gitBranchFromEnvironment } from "./lib/envvars.js";
import { devAgainstDeployment } from "./lib/dev.js";
import { handleLocalDeployment } from "./lib/localDeployment/localDeployment.js";
import {
  validateOrSelectTeam,
  validateOrSelectProject,
} from "./lib/utils/utils.js";
import { ensureLoggedIn } from "./lib/login.js";

vi.mock("../bundler/fs.js", async (importOriginal) => {
  const actual = await importOriginal<typeof import("../bundler/fs.js")>();
  return {
    ...actual,
    nodeFs: {
      ...actual.nodeFs,
      exists: vi.fn().mockImplementation(() => {
        throw new Error("nodeFs.exists should be mocked in test");
      }),
      readUtf8File: vi.fn().mockImplementation(() => {
        throw new Error("nodeFs.readUtf8File should be mocked in test");
      }),
    },
  };
});

vi.mock("./lib/utils/utils.js", async (importOriginal) => {
  const actual = await importOriginal<typeof import("./lib/utils/utils.js")>();
  return {
    ...actual,
    deploymentFetch: vi.fn(),
    ensureHasConvexDependency: vi.fn(),
    bigBrainAPI: vi.fn(),
    bigBrainAPIMaybeThrows: vi.fn(),
    validateOrSelectTeam: vi.fn(),
    validateOrSelectProject: vi.fn(),
  };
});

vi.mock("./lib/localDeployment/run.js", async (importOriginal) => {
  const actual =
    await importOriginal<typeof import("./lib/localDeployment/run.js")>();
  return {
    ...actual,
    withRunningBackend: vi.fn(
      async ({ action }: { action: () => Promise<void> }) => {
        await action();
      },
    ),
  };
});

vi.mock("./lib/utils/globalConfig.js", async (importOriginal) => {
  const actual =
    await importOriginal<typeof import("./lib/utils/globalConfig.js")>();
  return {
    ...actual,
    readGlobalConfig: vi.fn().mockReturnValue(null),
  };
});

vi.mock("dotenv", async (importOriginal) => {
  const actual = await importOriginal<typeof import("dotenv")>();
  return {
    ...actual,
    config: vi.fn(),
  };
});

vi.mock("@sentry/node", () => ({
  captureException: vi.fn(),
  close: vi.fn(),
}));

// Deploy-specific mocks
vi.mock("./lib/deploy2.js", async (importOriginal) => {
  const actual = await importOriginal<typeof import("./lib/deploy2.js")>();
  return {
    ...actual,
    deployToDeployment: vi.fn(),
  };
});

vi.mock("./lib/components.js", async (importOriginal) => {
  const actual = await importOriginal<typeof import("./lib/components.js")>();
  return {
    ...actual,
    runPush: vi.fn(),
  };
});

vi.mock("./lib/config.js", async (importOriginal) => {
  const actual = await importOriginal<typeof import("./lib/config.js")>();
  return {
    ...actual,
    readProjectConfig: vi.fn().mockResolvedValue({
      projectConfig: {},
      configPath: "convex.json",
      modules: [],
    }),
    getAuthKitConfig: vi.fn().mockResolvedValue(null),
  };
});

vi.mock("./lib/usage.js", async (importOriginal) => {
  const actual = await importOriginal<typeof import("./lib/usage.js")>();
  return {
    ...actual,
    usageStateWarning: vi.fn(),
  };
});

vi.mock("./lib/updates.js", async (importOriginal) => {
  const actual = await importOriginal<typeof import("./lib/updates.js")>();
  return {
    ...actual,
    checkVersion: vi.fn(),
  };
});

vi.mock("./lib/dev.js", async (importOriginal) => {
  const actual = await importOriginal<typeof import("./lib/dev.js")>();
  return { ...actual, devAgainstDeployment: vi.fn() };
});

vi.mock("./lib/localDeployment/localDeployment.js", async (importOriginal) => {
  const actual =
    await importOriginal<
      typeof import("./lib/localDeployment/localDeployment.js")
    >();
  return { ...actual, handleLocalDeployment: vi.fn() };
});

vi.mock("./lib/login.js", async (importOriginal) => {
  const actual = await importOriginal<typeof import("./lib/login.js")>();
  return { ...actual, ensureLoggedIn: vi.fn() };
});

vi.mock("./configure.js", async (importOriginal) => {
  const actual = await importOriginal<typeof import("./configure.js")>();
  return {
    ...actual,
    // Override to skip fetchDeploymentCanonicalSiteUrl and file-writing side
    // effects, while still exercising _deploymentCredentialsOrConfigure (which
    // routes through the BigBrain mocks, handleLocalDeployment, etc.).
    deploymentCredentialsOrConfigure: async (
      ctx: any,
      deploymentSelection: any,
      chosenConfiguration: any,
      cmdOptions: any,
    ) => {
      const selected = await actual._deploymentCredentialsOrConfigure(
        ctx,
        deploymentSelection,
        chosenConfiguration,
        cmdOptions,
      );
      return {
        url: selected.url,
        adminKey: selected.adminKey,
        deploymentFields:
          selected.deploymentFields !== null
            ? { ...selected.deploymentFields, siteUrl: null }
            : null,
      };
    },
  };
});

vi.mock("./lib/envvars.js", async (importOriginal) => {
  const actual = await importOriginal<typeof import("./lib/envvars.js")>();
  return {
    ...actual,
    gitBranchFromEnvironment: vi.fn().mockReturnValue(null),
    isNonProdBuildEnvironment: vi.fn().mockReturnValue(false),
  };
});

/**
 * Routes mock Big Brain API calls by path.
 * Both `bigBrainAPI` and `bigBrainAPIMaybeThrows` delegate to this.
 */
function setupBigBrainRoutes(routes: Record<string, (data?: any) => any>) {
  const handler = (args: { path: string; data?: any }) => {
    // Match by exact path or by path prefix (for paths like `deployment/foo/team_and_project`)
    for (const [routePath, routeHandler] of Object.entries(routes)) {
      if (args.path === routePath || args.path.startsWith(routePath)) {
        return routeHandler(args.data);
      }
    }
    throw new Error(`Unmocked Big Brain route: ${args.path}`);
  };
  vi.mocked(bigBrainAPI).mockImplementation(handler as any);
  vi.mocked(bigBrainAPIMaybeThrows).mockImplementation(handler as any);
}

describe("deployment selection flows", () => {
  let savedEnv: NodeJS.ProcessEnv;

  beforeEach(() => {
    savedEnv = { ...process.env };
    process.env = {};

    vi.resetAllMocks();
    vi.mocked(readGlobalConfig).mockReturnValue(null);
    vi.mocked(nodeFs.exists).mockReturnValue(false);
    // Re-apply deploy-specific mocks after resetAllMocks
    vi.mocked(readProjectConfig).mockResolvedValue({
      projectConfig: {} as any,
      configPath: "convex.json",
    });
    vi.mocked(getAuthKitConfig).mockResolvedValue(undefined);
    vi.mocked(gitBranchFromEnvironment).mockReturnValue(null);
    vi.mocked(devAgainstDeployment).mockResolvedValue(undefined);
    vi.mocked(handleLocalDeployment).mockResolvedValue({
      deploymentName: "local-test",
      deploymentUrl: "http://127.0.0.1:3210",
      adminKey: "local|admin|key",
      onActivity: async () => {},
    } as any);
    vi.mocked(validateOrSelectTeam).mockRejectedValue(
      new Error("validateOrSelectTeam should be mocked"),
    );
    vi.mocked(validateOrSelectProject).mockRejectedValue(
      new Error("validateOrSelectProject should be mocked"),
    );
    vi.mocked(ensureLoggedIn).mockResolvedValue(undefined);
  });

  afterEach(() => {
    process.env = savedEnv;
  });

  // Suppress process.exit and stderr
  beforeEach(() => {
    vi.spyOn(process, "exit").mockImplementation((() => {
      throw new Error("process.exit called");
    }) as any);
    vi.spyOn(process.stderr, "write").mockImplementation(() => true);
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  describe("regular command (npx convex env)", () => {
    it("uses --url and --admin-key directly", async () => {
      const mockFetch = vi.fn().mockResolvedValue({ ok: true });
      vi.mocked(deploymentFetch).mockReturnValue(mockFetch as any);

      await env.parseAsync(
        [
          "set",
          "ABC",
          "DEF",
          "--url",
          "https://joyful-capybara-123.convex.cloud",
          "--admin-key",
          "my-admin-key",
        ],
        { from: "user" },
      );

      expect(deploymentFetch).toHaveBeenCalledWith(
        expect.anything(),
        expect.objectContaining({
          deploymentUrl: "https://joyful-capybara-123.convex.cloud",
          adminKey: "my-admin-key",
        }),
      );

      expect(mockFetch).toHaveBeenCalledWith(
        "/api/update_environment_variables",
        expect.objectContaining({
          method: "POST",
          body: JSON.stringify({ changes: [{ name: "ABC", value: "DEF" }] }),
        }),
      );

      // No Big Brain calls
      expect(bigBrainAPI).not.toHaveBeenCalled();
      expect(bigBrainAPIMaybeThrows).not.toHaveBeenCalled();
    });

    it("resolves CONVEX_DEPLOY_KEY with deployment deploy key via Big Brain", async () => {
      process.env.CONVEX_DEPLOY_KEY = "prod:joyful-capybara-123|secretkey";

      setupBigBrainRoutes({
        "deployment/url_for_key": () =>
          "https://joyful-capybara-123.eu-west-1.convex.cloud",
        "deployment/team_and_project_for_key": () => ({
          team: "my-team",
          project: "my-project",
          teamId: 1,
          projectId: 1,
        }),
      });

      const mockFetch = vi.fn().mockResolvedValue({ ok: true });
      vi.mocked(deploymentFetch).mockReturnValue(mockFetch as any);

      await env.parseAsync(["set", "ABC", "DEF"], { from: "user" });

      expect(deploymentFetch).toHaveBeenCalledWith(
        expect.anything(),
        expect.objectContaining({
          deploymentUrl: "https://joyful-capybara-123.eu-west-1.convex.cloud",
          adminKey: "prod:joyful-capybara-123|secretkey",
        }),
      );

      expect(mockFetch).toHaveBeenCalledWith(
        "/api/update_environment_variables",
        expect.objectContaining({
          method: "POST",
          body: JSON.stringify({ changes: [{ name: "ABC", value: "DEF" }] }),
        }),
      );

      expect(bigBrainAPI).toHaveBeenCalledWith(
        expect.objectContaining({ path: "deployment/url_for_key" }),
      );
      expect(bigBrainAPI).toHaveBeenCalledWith(
        expect.objectContaining({
          path: "deployment/team_and_project_for_key",
        }),
      );
    });

    it("resolves CONVEX_DEPLOY_KEY with project deploy key to dev deployment by default", async () => {
      process.env.CONVEX_DEPLOY_KEY = "project:identifier|secretkey";

      setupBigBrainRoutes({
        "deployment/provision_and_authorize": () => ({
          adminKey: "dev-admin-key",
          url: "https://swift-squirrel-234.convex.cloud",
          deploymentName: "swift-squirrel-234",
        }),
      });

      const mockFetch = vi.fn().mockResolvedValue({ ok: true });
      vi.mocked(deploymentFetch).mockReturnValue(mockFetch as any);

      await env.parseAsync(["set", "ABC", "DEF"], { from: "user" });

      expect(deploymentFetch).toHaveBeenCalledWith(
        expect.anything(),
        expect.objectContaining({
          deploymentUrl: "https://swift-squirrel-234.convex.cloud",
          adminKey: "dev-admin-key",
        }),
      );

      expect(mockFetch).toHaveBeenCalledWith(
        "/api/update_environment_variables",
        expect.objectContaining({
          method: "POST",
          body: JSON.stringify({ changes: [{ name: "ABC", value: "DEF" }] }),
        }),
      );

      expect(bigBrainAPIMaybeThrows).toHaveBeenCalledWith(
        expect.objectContaining({
          path: "deployment/provision_and_authorize",
        }),
      );
    });

    it("resolves CONVEX_DEPLOY_KEY with project deploy key to prod deployment with --prod", async () => {
      process.env.CONVEX_DEPLOY_KEY = "project:identifier|secretkey";

      setupBigBrainRoutes({
        "deployment/provision_and_authorize": (_data: any) => ({
          adminKey: "prod-admin-key",
          url: "https://graceful-puffin-456.convex.cloud",
          deploymentName: "graceful-puffin-456",
        }),
      });

      const mockFetch = vi.fn().mockResolvedValue({ ok: true });
      vi.mocked(deploymentFetch).mockReturnValue(mockFetch as any);

      await env.parseAsync(["set", "ABC", "DEF", "--prod"], { from: "user" });

      expect(deploymentFetch).toHaveBeenCalledWith(
        expect.anything(),
        expect.objectContaining({
          deploymentUrl: "https://graceful-puffin-456.convex.cloud",
          adminKey: "prod-admin-key",
        }),
      );

      expect(mockFetch).toHaveBeenCalledWith(
        "/api/update_environment_variables",
        expect.objectContaining({
          method: "POST",
          body: JSON.stringify({ changes: [{ name: "ABC", value: "DEF" }] }),
        }),
      );

      // provision_and_authorize is called with prod deploymentType
      expect(bigBrainAPIMaybeThrows).toHaveBeenCalledWith(
        expect.objectContaining({
          path: "deployment/provision_and_authorize",
          data: expect.objectContaining({
            deploymentType: "prod",
          }),
        }),
      );
    });

    it("uses CONVEX_SELF_HOSTED_URL and CONVEX_SELF_HOSTED_ADMIN_KEY directly", async () => {
      process.env.CONVEX_SELF_HOSTED_URL = "http://localhost:3210";
      process.env.CONVEX_SELF_HOSTED_ADMIN_KEY = "self-hosted-key";

      const mockFetch = vi.fn().mockResolvedValue({ ok: true });
      vi.mocked(deploymentFetch).mockReturnValue(mockFetch as any);

      await env.parseAsync(["set", "ABC", "DEF"], { from: "user" });

      expect(deploymentFetch).toHaveBeenCalledWith(
        expect.anything(),
        expect.objectContaining({
          deploymentUrl: "http://localhost:3210",
          adminKey: "self-hosted-key",
        }),
      );

      expect(mockFetch).toHaveBeenCalledWith(
        "/api/update_environment_variables",
        expect.objectContaining({
          method: "POST",
          body: JSON.stringify({ changes: [{ name: "ABC", value: "DEF" }] }),
        }),
      );

      // No Big Brain calls
      expect(bigBrainAPI).not.toHaveBeenCalled();
      expect(bigBrainAPIMaybeThrows).not.toHaveBeenCalled();
    });

    it("resolves CONVEX_DEPLOYMENT to dev deployment by default", async () => {
      process.env.CONVEX_DEPLOYMENT = "dev:joyful-capybara-123";
      vi.mocked(readGlobalConfig).mockReturnValue({
        accessToken: "test-token",
      });

      setupBigBrainRoutes({
        "deployment/joyful-capybara-123/team_and_project": () => ({
          team: "my-team",
          project: "my-project",
          teamId: 1,
          projectId: 1,
        }),
        "deployment/provision_and_authorize": () => ({
          adminKey: "dev-key",
          url: "https://swift-squirrel-234.convex.cloud",
          deploymentName: "swift-squirrel-234",
        }),
      });

      const mockFetch = vi.fn().mockResolvedValue({ ok: true });
      vi.mocked(deploymentFetch).mockReturnValue(mockFetch as any);

      await env.parseAsync(["set", "ABC", "DEF"], { from: "user" });

      expect(deploymentFetch).toHaveBeenCalledWith(
        expect.anything(),
        expect.objectContaining({
          deploymentUrl: "https://swift-squirrel-234.convex.cloud",
          adminKey: "dev-key",
        }),
      );

      expect(mockFetch).toHaveBeenCalledWith(
        "/api/update_environment_variables",
        expect.objectContaining({
          method: "POST",
          body: JSON.stringify({ changes: [{ name: "ABC", value: "DEF" }] }),
        }),
      );

      // checkAccessToSelectedProject calls getTeamAndProjectSlugForDeployment
      expect(bigBrainAPIMaybeThrows).toHaveBeenCalledWith(
        expect.objectContaining({
          path: "deployment/joyful-capybara-123/team_and_project",
          method: "GET",
        }),
      );
    });

    it("resolves CONVEX_DEPLOYMENT with --prod to prod deployment", async () => {
      process.env.CONVEX_DEPLOYMENT = "dev:joyful-capybara-123";
      vi.mocked(readGlobalConfig).mockReturnValue({
        accessToken: "test-token",
      });

      setupBigBrainRoutes({
        "deployment/joyful-capybara-123/team_and_project": () => ({
          team: "my-team",
          project: "my-project",
          teamId: 1,
          projectId: 1,
        }),
        "deployment/authorize_prod": () => ({
          adminKey: "prod-key",
          url: "https://graceful-puffin-456.convex.cloud",
          deploymentName: "graceful-puffin-456",
          deploymentType: "prod",
        }),
      });

      const mockFetch = vi.fn().mockResolvedValue({ ok: true });
      vi.mocked(deploymentFetch).mockReturnValue(mockFetch as any);

      await env.parseAsync(["set", "ABC", "DEF", "--prod"], { from: "user" });

      expect(deploymentFetch).toHaveBeenCalledWith(
        expect.anything(),
        expect.objectContaining({
          deploymentUrl: "https://graceful-puffin-456.convex.cloud",
          adminKey: "prod-key",
        }),
      );

      expect(mockFetch).toHaveBeenCalledWith(
        "/api/update_environment_variables",
        expect.objectContaining({
          method: "POST",
          body: JSON.stringify({ changes: [{ name: "ABC", value: "DEF" }] }),
        }),
      );

      expect(bigBrainAPI).toHaveBeenCalledWith(
        expect.objectContaining({
          path: "deployment/authorize_prod",
        }),
      );
    });

    it("resolves CONVEX_DEPLOYMENT with --preview-name to preview deployment", async () => {
      process.env.CONVEX_DEPLOYMENT = "dev:joyful-capybara-123";
      vi.mocked(readGlobalConfig).mockReturnValue({
        accessToken: "test-token",
      });

      setupBigBrainRoutes({
        "deployment/joyful-capybara-123/team_and_project": () => ({
          team: "my-team",
          project: "my-project",
          teamId: 1,
          projectId: 1,
        }),
        "deployment/authorize_preview": () => ({
          adminKey: "preview-key",
          url: "https://nimble-penguin-234.convex.cloud",
          deploymentName: "nimble-penguin-234",
          deploymentType: "preview",
        }),
      });

      const mockFetch = vi.fn().mockResolvedValue({ ok: true });
      vi.mocked(deploymentFetch).mockReturnValue(mockFetch as any);

      await env.parseAsync(
        ["set", "ABC", "DEF", "--preview-name", "my-preview"],
        { from: "user" },
      );

      expect(deploymentFetch).toHaveBeenCalledWith(
        expect.anything(),
        expect.objectContaining({
          deploymentUrl: "https://nimble-penguin-234.convex.cloud",
          adminKey: "preview-key",
        }),
      );

      expect(mockFetch).toHaveBeenCalledWith(
        "/api/update_environment_variables",
        expect.objectContaining({
          method: "POST",
          body: JSON.stringify({ changes: [{ name: "ABC", value: "DEF" }] }),
        }),
      );

      expect(bigBrainAPI).toHaveBeenCalledWith(
        expect.objectContaining({
          path: "deployment/authorize_preview",
        }),
      );
    });

    it("resolves CONVEX_DEPLOYMENT with --deployment-name to named deployment", async () => {
      process.env.CONVEX_DEPLOYMENT = "dev:joyful-capybara-123";
      vi.mocked(readGlobalConfig).mockReturnValue({
        accessToken: "test-token",
      });

      setupBigBrainRoutes({
        "deployment/limitless-wolf-571/team_and_project": () => ({
          team: "my-team",
          project: "my-project",
          teamId: 1,
          projectId: 1,
        }),
        "deployment/authorize_within_current_project": () => ({
          adminKey: "staging-key",
          url: "https://clever-otter-890.convex.cloud",
          deploymentName: "clever-otter-890",
          deploymentType: "dev",
        }),
      });

      const mockFetch = vi.fn().mockResolvedValue({ ok: true });
      vi.mocked(deploymentFetch).mockReturnValue(mockFetch as any);

      await env.parseAsync(
        ["set", "ABC", "DEF", "--deployment-name", "limitless-wolf-571"],
        { from: "user" },
      );

      expect(deploymentFetch).toHaveBeenCalledWith(
        expect.anything(),
        expect.objectContaining({
          deploymentUrl: "https://clever-otter-890.convex.cloud",
          adminKey: "staging-key",
        }),
      );

      expect(mockFetch).toHaveBeenCalledWith(
        "/api/update_environment_variables",
        expect.objectContaining({
          method: "POST",
          body: JSON.stringify({ changes: [{ name: "ABC", value: "DEF" }] }),
        }),
      );

      expect(bigBrainAPI).toHaveBeenCalledWith(
        expect.objectContaining({
          path: "deployment/authorize_within_current_project",
          data: expect.objectContaining({
            projectSelection: expect.objectContaining({
              kind: "deploymentName",
              deploymentName: "limitless-wolf-571",
              deploymentType: null,
            }),
          }),
        }),
      );
    });

    it("resolves --deployment-name targeting a deployment in a different project from CONVEX_DEPLOYMENT", async () => {
      process.env.CONVEX_DEPLOYMENT = "dev:joyful-capybara-123";
      vi.mocked(readGlobalConfig).mockReturnValue({
        accessToken: "test-token",
      });

      setupBigBrainRoutes({
        "deployment/cross-project-deploy/team_and_project": () => ({
          team: "other-team",
          project: "other-project",
          teamId: 2,
          projectId: 2,
        }),
        "deployment/authorize_within_current_project": () => ({
          adminKey: "cross-project-key",
          url: "https://cross-project-deploy.convex.cloud",
          deploymentName: "cross-project-deploy",
          deploymentType: "dev",
        }),
      });

      const mockFetch = vi.fn().mockResolvedValue({ ok: true });
      vi.mocked(deploymentFetch).mockReturnValue(mockFetch as any);

      await env.parseAsync(
        ["set", "ABC", "DEF", "--deployment-name", "cross-project-deploy"],
        { from: "user" },
      );

      expect(deploymentFetch).toHaveBeenCalledWith(
        expect.anything(),
        expect.objectContaining({
          deploymentUrl: "https://cross-project-deploy.convex.cloud",
          adminKey: "cross-project-key",
        }),
      );

      // Verify authorize_within_current_project was called with the
      // --deployment-name deployment as the project selector, not CONVEX_DEPLOYMENT
      expect(bigBrainAPI).toHaveBeenCalledWith(
        expect.objectContaining({
          path: "deployment/authorize_within_current_project",
          data: expect.objectContaining({
            projectSelection: expect.objectContaining({
              kind: "deploymentName",
              deploymentName: "cross-project-deploy",
              deploymentType: null,
            }),
            selectedDeploymentName: "cross-project-deploy",
          }),
        }),
      );
    });

    it("resolves --deployment-name with cloud deployment name without CONVEX_DEPLOYMENT", async () => {
      delete process.env.CONVEX_DEPLOYMENT;

      setupBigBrainRoutes({
        "deployment/clever-otter-890/team_and_project": () => ({
          team: "my-team",
          project: "my-project",
          teamId: 1,
          projectId: 1,
        }),
        "deployment/authorize_within_current_project": () => ({
          adminKey: "other-key",
          url: "https://clever-otter-890.convex.cloud",
          deploymentName: "clever-otter-890",
          deploymentType: "dev",
        }),
      });

      const mockFetch = vi.fn().mockResolvedValue({ ok: true });
      vi.mocked(deploymentFetch).mockReturnValue(mockFetch as any);

      await env.parseAsync(
        ["set", "ABC", "DEF", "--deployment-name", "clever-otter-890"],
        { from: "user" },
      );

      // The project was resolved using clever-otter-890 as the anchor, not
      // joyful-capybara-123 (which isn't set in this test).
      expect(bigBrainAPIMaybeThrows).toHaveBeenCalledWith(
        expect.objectContaining({
          path: "deployment/clever-otter-890/team_and_project",
        }),
      );
      expect(bigBrainAPI).toHaveBeenCalledWith(
        expect.objectContaining({
          path: "deployment/authorize_within_current_project",
        }),
      );
      expect(deploymentFetch).toHaveBeenCalledWith(
        expect.anything(),
        expect.objectContaining({
          deploymentUrl: "https://clever-otter-890.convex.cloud",
          adminKey: "other-key",
        }),
      );
    });

    it("resolves deployment deploy key from --env-file", async () => {
      const fakeEnvFilePath = "/fake/convex-test.env";
      vi.mocked(nodeFs.exists).mockReturnValue(true);
      vi.mocked(nodeFs.readUtf8File).mockReturnValue(
        "CONVEX_DEPLOY_KEY=prod:joyful-capybara-123|secretkey\n",
      );

      setupBigBrainRoutes({
        "deployment/url_for_key": () =>
          "https://joyful-capybara-123.convex.cloud",
        "deployment/team_and_project_for_key": () => ({
          team: "my-team",
          project: "my-project",
          teamId: 1,
          projectId: 1,
        }),
      });

      const mockFetch = vi.fn().mockResolvedValue({ ok: true });
      vi.mocked(deploymentFetch).mockReturnValue(mockFetch as any);

      await env.parseAsync(
        ["set", "ABC", "DEF", "--env-file", fakeEnvFilePath],
        {
          from: "user",
        },
      );

      expect(deploymentFetch).toHaveBeenCalledWith(
        expect.anything(),
        expect.objectContaining({
          deploymentUrl: "https://joyful-capybara-123.convex.cloud",
          adminKey: "prod:joyful-capybara-123|secretkey",
        }),
      );

      expect(mockFetch).toHaveBeenCalledWith(
        "/api/update_environment_variables",
        expect.objectContaining({
          method: "POST",
          body: JSON.stringify({ changes: [{ name: "ABC", value: "DEF" }] }),
        }),
      );

      expect(bigBrainAPI).toHaveBeenCalledWith(
        expect.objectContaining({ path: "deployment/url_for_key" }),
      );
      expect(bigBrainAPI).toHaveBeenCalledWith(
        expect.objectContaining({
          path: "deployment/team_and_project_for_key",
        }),
      );
    });
  });

  describe("deploy command (npx convex deploy)", () => {
    it("defaults to prod with CONVEX_DEPLOYMENT (implicitProd)", async () => {
      process.env.CONVEX_DEPLOYMENT = "dev:joyful-capybara-123";
      vi.mocked(readGlobalConfig).mockReturnValue({
        accessToken: "test-token",
      });

      setupBigBrainRoutes({
        "deployment/joyful-capybara-123/team_and_project": () => ({
          team: "my-team",
          project: "my-project",
          teamId: 1,
          projectId: 1,
        }),
        "deployment/authorize_prod": () => ({
          adminKey: "prod-key",
          url: "https://graceful-puffin-456.convex.cloud",
          deploymentName: "graceful-puffin-456",
          deploymentType: "prod",
        }),
      });

      await deploy.parseAsync(["--yes"], { from: "user" });

      expect(bigBrainAPI).toHaveBeenCalledWith(
        expect.objectContaining({
          path: "deployment/authorize_prod",
        }),
      );

      expect(deployToDeployment).toHaveBeenCalledWith(
        expect.anything(),
        expect.objectContaining({
          url: "https://graceful-puffin-456.convex.cloud",
          adminKey: "prod-key",
        }),
        expect.anything(),
      );
    });

    it("defaults to prod with project deploy key", async () => {
      process.env.CONVEX_DEPLOY_KEY = "project:identifier|secretkey";

      setupBigBrainRoutes({
        "deployment/provision_and_authorize": () => ({
          adminKey: "prod-admin-key",
          url: "https://graceful-puffin-456.convex.cloud",
          deploymentName: "graceful-puffin-456",
        }),
      });

      await deploy.parseAsync([], { from: "user" });

      expect(bigBrainAPIMaybeThrows).toHaveBeenCalledWith(
        expect.objectContaining({
          path: "deployment/provision_and_authorize",
          data: expect.objectContaining({
            deploymentType: "prod",
          }),
        }),
      );

      expect(deployToDeployment).toHaveBeenCalledWith(
        expect.anything(),
        expect.objectContaining({
          url: "https://graceful-puffin-456.convex.cloud",
          adminKey: "prod-admin-key",
        }),
        expect.anything(),
      );
    });

    it("deploys to preview with preview deploy key and --preview-create", async () => {
      process.env.CONVEX_DEPLOY_KEY = "preview:my-team:my-project|secretkey";

      setupBigBrainRoutes({
        claim_preview_deployment: () => ({
          adminKey: "preview-admin-key",
          instanceUrl: "https://nimble-penguin-234.convex.cloud",
        }),
      });

      await deploy.parseAsync(["--preview-create", "my-preview"], {
        from: "user",
      });

      expect(bigBrainAPI).toHaveBeenCalledWith(
        expect.objectContaining({
          path: "claim_preview_deployment",
          data: expect.objectContaining({
            identifier: "my-preview",
            projectSelection: {
              kind: "teamAndProjectSlugs",
              teamSlug: "my-team",
              projectSlug: "my-project",
            },
          }),
        }),
      );

      expect(runPush).toHaveBeenCalledWith(
        expect.anything(),
        expect.objectContaining({
          url: "https://nimble-penguin-234.convex.cloud",
          adminKey: "preview-admin-key",
        }),
      );
    });

    it("deploys to preview with preview deploy key using git branch fallback", async () => {
      process.env.CONVEX_DEPLOY_KEY = "preview:my-team:my-project|secretkey";
      vi.mocked(gitBranchFromEnvironment).mockReturnValue("feature/my-branch");

      setupBigBrainRoutes({
        claim_preview_deployment: () => ({
          adminKey: "preview-admin-key",
          instanceUrl: "https://nimble-penguin-234.convex.cloud",
        }),
      });

      await deploy.parseAsync([], { from: "user" });

      expect(bigBrainAPI).toHaveBeenCalledWith(
        expect.objectContaining({
          path: "claim_preview_deployment",
          data: expect.objectContaining({
            identifier: "feature/my-branch",
          }),
        }),
      );

      expect(runPush).toHaveBeenCalled();
    });

    it("deploys to existing preview with CONVEX_DEPLOYMENT and --preview-name", async () => {
      process.env.CONVEX_DEPLOYMENT = "dev:joyful-capybara-123";
      vi.mocked(readGlobalConfig).mockReturnValue({
        accessToken: "test-token",
      });

      setupBigBrainRoutes({
        "deployment/joyful-capybara-123/team_and_project": () => ({
          team: "my-team",
          project: "my-project",
          teamId: 1,
          projectId: 1,
        }),
        "deployment/authorize_preview": () => ({
          adminKey: "preview-key",
          url: "https://nimble-penguin-234.convex.cloud",
          deploymentName: "nimble-penguin-234",
          deploymentType: "preview",
        }),
      });

      await deploy.parseAsync(["--preview-name", "my-preview", "--yes"], {
        from: "user",
      });

      expect(bigBrainAPI).toHaveBeenCalledWith(
        expect.objectContaining({
          path: "deployment/authorize_preview",
        }),
      );

      expect(deployToDeployment).toHaveBeenCalledWith(
        expect.anything(),
        expect.objectContaining({
          url: "https://nimble-penguin-234.convex.cloud",
          adminKey: "preview-key",
        }),
        expect.anything(),
      );
    });

    it("crashes with preview deploy key and --preview-name (deprecated)", async () => {
      process.env.CONVEX_DEPLOY_KEY = "preview:my-team:my-project|secretkey";

      await expect(
        deploy.parseAsync(["--preview-name", "my-preview"], { from: "user" }),
      ).rejects.toThrow();

      expect(deployToDeployment).not.toHaveBeenCalled();
      expect(runPush).not.toHaveBeenCalled();
    });
  });

  describe("dev command (npx convex dev)", () => {
    it("dev --local uses local deployment when local is allowed", async () => {
      vi.mocked(readGlobalConfig).mockReturnValue({
        accessToken: "test-token",
      }); // no optOutOfLocalDevDeploymentsUntilBetaOver
      vi.mocked(validateOrSelectTeam).mockResolvedValue({
        team: { slug: "my-team", id: 1, name: "My Team" } as any,
        chosen: false,
      });
      vi.mocked(validateOrSelectProject).mockResolvedValue("my-project");

      await dev.parseAsync(["--local", "--configure", "existing"], {
        from: "user",
      });

      expect(handleLocalDeployment).toHaveBeenCalledWith(
        expect.anything(),
        expect.objectContaining({
          teamSlug: "my-team",
          projectSlug: "my-project",
        }),
      );
      expect(devAgainstDeployment).toHaveBeenCalledWith(
        expect.anything(),
        expect.objectContaining({
          url: "http://127.0.0.1:3210",
          adminKey: "local|admin|key",
        }),
        expect.anything(),
      );
      expect(bigBrainAPI).not.toHaveBeenCalled();
      expect(bigBrainAPIMaybeThrows).not.toHaveBeenCalled();
    });

    it("dev --local crashes when local deployments are globally disabled", async () => {
      vi.mocked(readGlobalConfig).mockReturnValue({
        accessToken: "test-token",
        optOutOfLocalDevDeploymentsUntilBetaOver: true,
      });

      await expect(
        dev.parseAsync(["--skip-push", "--local"], { from: "user" }),
      ).rejects.toThrow();

      expect(process.stderr.write).toHaveBeenCalledWith(
        expect.stringContaining(
          "Can't specify --local when local deployments are disabled on this machine",
        ),
      );
      expect(devAgainstDeployment).not.toHaveBeenCalled();
    });

    it("defaults to dev deployment with CONVEX_DEPLOYMENT", async () => {
      process.env.CONVEX_DEPLOYMENT = "dev:joyful-capybara-123";
      vi.mocked(readGlobalConfig).mockReturnValue({
        accessToken: "test-token",
      });

      setupBigBrainRoutes({
        "deployment/joyful-capybara-123/team_and_project": () => ({
          team: "my-team",
          project: "my-project",
          teamId: 1,
          projectId: 1,
        }),
        "deployment/provision_and_authorize": () => ({
          adminKey: "dev-key",
          url: "https://joyful-capybara-123.convex.cloud",
          deploymentName: "joyful-capybara-123",
          deploymentType: "dev",
        }),
      });

      await dev.parseAsync([], { from: "user" });

      expect(bigBrainAPIMaybeThrows).toHaveBeenCalledWith(
        expect.objectContaining({
          path: "deployment/provision_and_authorize",
          data: expect.objectContaining({ deploymentType: "dev" }),
        }),
      );
      expect(devAgainstDeployment).toHaveBeenCalledWith(
        expect.anything(),
        expect.objectContaining({
          url: "https://joyful-capybara-123.convex.cloud",
          adminKey: "dev-key",
        }),
        expect.anything(),
      );
    });

    it("defaults to dev deployment with project deploy key", async () => {
      process.env.CONVEX_DEPLOY_KEY = "project:identifier|secretkey";

      setupBigBrainRoutes({
        "deployment/provision_and_authorize": () => ({
          adminKey: "dev-key",
          url: "https://swift-squirrel-234.convex.cloud",
          deploymentName: "swift-squirrel-234",
          deploymentType: "dev",
        }),
      });

      await dev.parseAsync([], { from: "user" });

      expect(bigBrainAPIMaybeThrows).toHaveBeenCalledWith(
        expect.objectContaining({
          path: "deployment/provision_and_authorize",
          data: expect.objectContaining({ deploymentType: "dev" }),
        }),
      );
      expect(devAgainstDeployment).toHaveBeenCalledWith(
        expect.anything(),
        expect.objectContaining({
          url: "https://swift-squirrel-234.convex.cloud",
          adminKey: "dev-key",
        }),
        expect.anything(),
      );
    });

    it("uses CONVEX_SELF_HOSTED_URL and CONVEX_SELF_HOSTED_ADMIN_KEY directly", async () => {
      process.env.CONVEX_SELF_HOSTED_URL = "http://localhost:3210";
      process.env.CONVEX_SELF_HOSTED_ADMIN_KEY = "self-hosted-key";

      await dev.parseAsync([], { from: "user" });

      expect(devAgainstDeployment).toHaveBeenCalledWith(
        expect.anything(),
        expect.objectContaining({
          url: "http://localhost:3210",
          adminKey: "self-hosted-key",
        }),
        expect.anything(),
      );
      expect(bigBrainAPI).not.toHaveBeenCalled();
      expect(bigBrainAPIMaybeThrows).not.toHaveBeenCalled();
    });

    it("uses --cloud flag with CONVEX_DEPLOYMENT to force cloud dev deployment", async () => {
      process.env.CONVEX_DEPLOYMENT = "dev:joyful-capybara-123";
      vi.mocked(readGlobalConfig).mockReturnValue({
        accessToken: "test-token",
      });

      setupBigBrainRoutes({
        "deployment/joyful-capybara-123/team_and_project": () => ({
          team: "my-team",
          project: "my-project",
          teamId: 1,
          projectId: 1,
        }),
        "deployment/provision_and_authorize": () => ({
          adminKey: "cloud-dev-key",
          url: "https://joyful-capybara-123.convex.cloud",
          deploymentName: "joyful-capybara-123",
          deploymentType: "dev",
        }),
      });

      await dev.parseAsync(["--cloud"], { from: "user" });

      expect(devAgainstDeployment).toHaveBeenCalledWith(
        expect.anything(),
        expect.objectContaining({
          url: "https://joyful-capybara-123.convex.cloud",
          adminKey: "cloud-dev-key",
        }),
        expect.anything(),
      );
    });
  });
});
