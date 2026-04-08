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
import {
  handleLocalDeployment,
  loadLocalDeploymentCredentials,
} from "./lib/localDeployment/localDeployment.js";
import { handleAnonymousDeployment } from "./lib/localDeployment/anonymous.js";
import { loadProjectLocalConfig } from "./lib/localDeployment/filePaths.js";
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

// Mock typedPlatformClient GET function — can be configured per test
const mockPlatformGet = vi.fn();

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
    typedPlatformClient: vi.fn(() => ({ GET: mockPlatformGet })),
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
    fetchLocalBackendStatus: vi.fn().mockResolvedValue({ kind: "running" }),
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
      projectConfig: { functions: "convex" },
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
    checkVersionAndAiFilesStaleness: vi.fn(),
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
  return {
    ...actual,
    handleLocalDeployment: vi.fn(),
    loadLocalDeploymentCredentials: vi.fn(),
  };
});

vi.mock("./lib/localDeployment/filePaths.js", async (importOriginal) => {
  const actual =
    await importOriginal<typeof import("./lib/localDeployment/filePaths.js")>();
  return {
    ...actual,
    loadProjectLocalConfig: vi.fn().mockReturnValue(null),
  };
});

vi.mock("./lib/localDeployment/anonymous.js", async (importOriginal) => {
  const actual =
    await importOriginal<typeof import("./lib/localDeployment/anonymous.js")>();
  return {
    ...actual,
    handleAnonymousDeployment: vi.fn(),
  };
});

vi.mock("./lib/login.js", async (importOriginal) => {
  const actual = await importOriginal<typeof import("./lib/login.js")>();
  return { ...actual, ensureLoggedIn: vi.fn() };
});

vi.mock("./lib/aiFiles/index.js", async (importOriginal) => {
  const actual =
    await importOriginal<typeof import("./lib/aiFiles/index.js")>();
  return { ...actual, attemptSetupAiFiles: vi.fn() };
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
  let savedIsTTY: boolean | undefined;

  beforeEach(() => {
    savedEnv = { ...process.env };
    savedIsTTY = process.stdin.isTTY;
    process.env = {};
    // Default to interactive TTY for existing tests
    process.stdin.isTTY = true as any;

    vi.resetAllMocks();
    vi.mocked(readGlobalConfig).mockReturnValue(null);
    vi.mocked(nodeFs.exists).mockReturnValue(false);
    // Re-apply deploy-specific mocks after resetAllMocks
    vi.mocked(readProjectConfig).mockResolvedValue({
      projectConfig: { functions: "convex" } as any,
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
    vi.mocked(loadLocalDeploymentCredentials).mockResolvedValue({
      deploymentName: "local-test",
      deploymentUrl: "http://127.0.0.1:3210",
      adminKey: "local|admin|key",
    });
    vi.mocked(validateOrSelectTeam).mockRejectedValue(
      new Error("validateOrSelectTeam should be mocked"),
    );
    vi.mocked(validateOrSelectProject).mockRejectedValue(
      new Error("validateOrSelectProject should be mocked"),
    );
    vi.mocked(ensureLoggedIn).mockResolvedValue(undefined);
    vi.mocked(handleAnonymousDeployment).mockResolvedValue({
      deploymentName: "anon-test",
      deploymentUrl: "http://127.0.0.1:3210",
      adminKey: "anon|admin|key",
      onActivity: async () => {},
    });
  });

  afterEach(() => {
    process.env = savedEnv;
    process.stdin.isTTY = savedIsTTY as any;
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

    it("resolves CONVEX_DEPLOYMENT to CONVEX_DEPLOYMENT by default", async () => {
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
        "deployment/authorize_within_current_project": () => ({
          adminKey: "dev-key",
          url: "https://joyful-capybara-123.convex.cloud",
          deploymentName: "joyful-capybara-123",
          deploymentType: "dev",
        }),
      });

      const mockFetch = vi.fn().mockResolvedValue({ ok: true });
      vi.mocked(deploymentFetch).mockReturnValue(mockFetch as any);

      await env.parseAsync(["set", "ABC", "DEF"], { from: "user" });

      expect(deploymentFetch).toHaveBeenCalledWith(
        expect.anything(),
        expect.objectContaining({
          deploymentUrl: "https://joyful-capybara-123.convex.cloud",
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

      expect(bigBrainAPI).toHaveBeenCalledWith(
        expect.objectContaining({
          path: "deployment/authorize_within_current_project",
          data: expect.objectContaining({
            selectedDeploymentName: "joyful-capybara-123",
            projectSelection: expect.objectContaining({
              kind: "deploymentName",
              deploymentName: "joyful-capybara-123",
            }),
          }),
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

    describe("--deployment flag", () => {
      beforeEach(() => {
        process.env.CONVEX_DEPLOYMENT = "dev:joyful-capybara-123";
        vi.mocked(readGlobalConfig).mockReturnValue({
          accessToken: "test-token",
        });
      });

      it("resolves --deployment prod to production deployment", async () => {
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

        await env.parseAsync(["set", "ABC", "DEF", "--deployment", "prod"], {
          from: "user",
        });

        expect(bigBrainAPI).toHaveBeenCalledWith(
          expect.objectContaining({ path: "deployment/authorize_prod" }),
        );
        expect(deploymentFetch).toHaveBeenCalledWith(
          expect.anything(),
          expect.objectContaining({
            deploymentUrl: "https://graceful-puffin-456.convex.cloud",
            adminKey: "prod-key",
          }),
        );
      });

      it("resolves --deployment dev to dev deployment", async () => {
        setupBigBrainRoutes({
          "deployment/joyful-capybara-123/team_and_project": () => ({
            team: "my-team",
            project: "my-project",
            teamId: 1,
            projectId: 1,
          }),
          "teams/my-team/projects/my-project/deployments": () => true,
          "deployment/authorize_within_current_project": () => ({
            adminKey: "dev-key",
            url: "https://joyful-capybara-123.convex.cloud",
            deploymentName: "joyful-capybara-123",
            deploymentType: "dev",
          }),
        });
        mockPlatformGet.mockResolvedValue({
          data: { name: "joyful-capybara-123" },
          error: undefined,
        });

        const mockFetch = vi.fn().mockResolvedValue({ ok: true });
        vi.mocked(deploymentFetch).mockReturnValue(mockFetch as any);

        await env.parseAsync(["set", "ABC", "DEF", "--deployment", "dev"], {
          from: "user",
        });

        expect(mockPlatformGet).toHaveBeenCalledWith(
          "/teams/{team_id_or_slug}/projects/{project_slug}/deployment",
          expect.objectContaining({
            params: expect.objectContaining({
              path: {
                team_id_or_slug: "my-team",
                project_slug: "my-project",
              },
              query: { defaultDev: true },
            }),
          }),
        );
        expect(bigBrainAPI).toHaveBeenCalledWith(
          expect.objectContaining({
            path: "deployment/authorize_within_current_project",
            data: expect.objectContaining({
              selectedDeploymentName: "joyful-capybara-123",
            }),
          }),
        );
      });

      it("resolves --deployment with cloud deployment name (abc-xyz-123)", async () => {
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
          ["set", "ABC", "DEF", "--deployment", "clever-otter-890"],
          { from: "user" },
        );

        expect(bigBrainAPI).toHaveBeenCalledWith(
          expect.objectContaining({
            path: "deployment/authorize_within_current_project",
            data: expect.objectContaining({
              selectedDeploymentName: "clever-otter-890",
            }),
          }),
        );
      });

      it("resolves --deployment with cloud deployment name without CONVEX_DEPLOYMENT", async () => {
        // No CONVEX_DEPLOYMENT set — the deployment name itself must be used as the
        // project anchor (team_and_project is looked up via clever-otter-890, not
        // via some pre-existing CONVEX_DEPLOYMENT).
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
          ["set", "ABC", "DEF", "--deployment", "clever-otter-890"],
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
            data: expect.objectContaining({
              selectedDeploymentName: "clever-otter-890",
            }),
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

      it("resolves --deployment with a reference", async () => {
        setupBigBrainRoutes({
          "deployment/joyful-capybara-123/team_and_project": () => ({
            team: "my-team",
            project: "my-project",
            teamId: 1,
            projectId: 1,
          }),
          "teams/my-team/projects/my-project/deployments": () => true,
          "deployment/authorize_within_current_project": () => ({
            adminKey: "staging-key",
            url: "https://clever-otter-890.convex.cloud",
            deploymentName: "clever-otter-890",
            deploymentType: "dev",
          }),
        });
        mockPlatformGet.mockResolvedValue({
          data: { name: "clever-otter-890" },
          error: undefined,
        });

        const mockFetch = vi.fn().mockResolvedValue({ ok: true });
        vi.mocked(deploymentFetch).mockReturnValue(mockFetch as any);

        await env.parseAsync(["set", "ABC", "DEF", "--deployment", "staging"], {
          from: "user",
        });

        expect(mockPlatformGet).toHaveBeenCalledWith(
          "/teams/{team_id_or_slug}/projects/{project_slug}/deployment",
          expect.objectContaining({
            params: expect.objectContaining({
              path: { team_id_or_slug: "my-team", project_slug: "my-project" },
              query: { reference: "staging" },
            }),
          }),
        );
        expect(bigBrainAPI).toHaveBeenCalledWith(
          expect.objectContaining({
            path: "deployment/authorize_within_current_project",
            data: expect.objectContaining({
              selectedDeploymentName: "clever-otter-890",
            }),
          }),
        );
      });

      it("resolves --deployment with project:reference format", async () => {
        setupBigBrainRoutes({
          "deployment/joyful-capybara-123/team_and_project": () => ({
            team: "my-team",
            project: "my-project",
            teamId: 1,
            projectId: 1,
          }),
          "teams/my-team/projects/other-project/deployments": () => true,
          "deployment/authorize_within_current_project": () => ({
            adminKey: "other-proj-key",
            url: "https://other-deploy-123.convex.cloud",
            deploymentName: "other-deploy-123",
            deploymentType: "dev",
          }),
        });
        mockPlatformGet.mockResolvedValue({
          data: { name: "other-deploy-123" },
          error: undefined,
        });

        const mockFetch = vi.fn().mockResolvedValue({ ok: true });
        vi.mocked(deploymentFetch).mockReturnValue(mockFetch as any);

        await env.parseAsync(
          ["set", "ABC", "DEF", "--deployment", "other-project:staging"],
          { from: "user" },
        );

        expect(mockPlatformGet).toHaveBeenCalledWith(
          "/teams/{team_id_or_slug}/projects/{project_slug}/deployment",
          expect.objectContaining({
            params: expect.objectContaining({
              path: {
                team_id_or_slug: "my-team",
                project_slug: "other-project",
              },
              query: { reference: "staging" },
            }),
          }),
        );
        expect(bigBrainAPI).toHaveBeenCalledWith(
          expect.objectContaining({
            path: "deployment/authorize_within_current_project",
            data: expect.objectContaining({
              selectedDeploymentName: "other-deploy-123",
            }),
          }),
        );
      });

      it("resolves --deployment with team:project:reference format without CONVEX_DEPLOYMENT", async () => {
        // No CONVEX_DEPLOYMENT set
        delete process.env.CONVEX_DEPLOYMENT;
        setupBigBrainRoutes({
          "deployment/authorize_within_current_project": () => ({
            adminKey: "fq-key",
            url: "https://fully-qualified-123.convex.cloud",
            deploymentName: "fully-qualified-123",
            deploymentType: "dev",
          }),
          "teams/myteam/projects/myproj/deployments": () => true,
        });
        mockPlatformGet.mockResolvedValue({
          data: { name: "fully-qualified-123" },
          error: undefined,
        });

        const mockFetch = vi.fn().mockResolvedValue({ ok: true });
        vi.mocked(deploymentFetch).mockReturnValue(mockFetch as any);

        await env.parseAsync(
          ["set", "ABC", "DEF", "--deployment", "myteam:myproj:staging"],
          { from: "user" },
        );

        expect(mockPlatformGet).toHaveBeenCalledWith(
          "/teams/{team_id_or_slug}/projects/{project_slug}/deployment",
          expect.objectContaining({
            params: expect.objectContaining({
              path: { team_id_or_slug: "myteam", project_slug: "myproj" },
              query: { reference: "staging" },
            }),
          }),
        );
        expect(bigBrainAPI).toHaveBeenCalledWith(
          expect.objectContaining({
            path: "deployment/authorize_within_current_project",
            data: expect.objectContaining({
              selectedDeploymentName: "fully-qualified-123",
            }),
          }),
        );
      });

      it("resolves --deployment project:dev to dev deployment in another project", async () => {
        setupBigBrainRoutes({
          "deployment/joyful-capybara-123/team_and_project": () => ({
            team: "my-team",
            project: "my-project",
            teamId: 1,
            projectId: 1,
          }),
          "teams/my-team/projects/other-project/deployments": () => true,
          "deployment/authorize_within_current_project": () => ({
            adminKey: "other-project-dev-key",
            url: "https://other-project-dev.convex.cloud",
            deploymentName: "other-project-dev",
            deploymentType: "dev",
          }),
        });
        mockPlatformGet.mockResolvedValue({
          data: { name: "other-project-dev" },
          error: undefined,
        });

        const mockFetch = vi.fn().mockResolvedValue({ ok: true });
        vi.mocked(deploymentFetch).mockReturnValue(mockFetch as any);

        await env.parseAsync(
          ["set", "ABC", "DEF", "--deployment", "other-project:dev"],
          { from: "user" },
        );

        expect(mockPlatformGet).toHaveBeenCalledWith(
          "/teams/{team_id_or_slug}/projects/{project_slug}/deployment",
          expect.objectContaining({
            params: expect.objectContaining({
              path: {
                team_id_or_slug: "my-team",
                project_slug: "other-project",
              },
              query: { defaultDev: true },
            }),
          }),
        );
        expect(bigBrainAPI).toHaveBeenCalledWith(
          expect.objectContaining({
            path: "deployment/authorize_within_current_project",
            data: expect.objectContaining({
              selectedDeploymentName: "other-project-dev",
            }),
          }),
        );
        expect(deploymentFetch).toHaveBeenCalledWith(
          expect.anything(),
          expect.objectContaining({
            deploymentUrl: "https://other-project-dev.convex.cloud",
            adminKey: "other-project-dev-key",
          }),
        );
      });

      it("resolves --deployment project:prod to prod deployment in another project", async () => {
        setupBigBrainRoutes({
          "deployment/joyful-capybara-123/team_and_project": () => ({
            team: "my-team",
            project: "my-project",
            teamId: 1,
            projectId: 1,
          }),
          "deployment/provision_and_authorize": () => ({
            adminKey: "other-project-prod-key",
            url: "https://other-project-prod.convex.cloud",
            deploymentName: "other-project-prod",
          }),
        });

        const mockFetch = vi.fn().mockResolvedValue({ ok: true });
        vi.mocked(deploymentFetch).mockReturnValue(mockFetch as any);

        await env.parseAsync(
          ["set", "ABC", "DEF", "--deployment", "other-project:prod"],
          { from: "user" },
        );

        expect(bigBrainAPIMaybeThrows).toHaveBeenCalledWith(
          expect.objectContaining({
            path: "deployment/provision_and_authorize",
            data: expect.objectContaining({
              teamSlug: "my-team",
              projectSlug: "other-project",
              deploymentType: "prod",
            }),
          }),
        );
        expect(deploymentFetch).toHaveBeenCalledWith(
          expect.anything(),
          expect.objectContaining({
            deploymentUrl: "https://other-project-prod.convex.cloud",
            adminKey: "other-project-prod-key",
          }),
        );
      });

      it("resolves --deployment team:project:prod to prod deployment in fully qualified team/project", async () => {
        setupBigBrainRoutes({
          "teams/myteam/projects/myproject/deployments": () => true,
          "deployment/provision_and_authorize": () => ({
            adminKey: "fq-prod-key",
            url: "https://fq-prod-deploy.convex.cloud",
            deploymentName: "fq-prod-deploy",
          }),
        });

        const mockFetch = vi.fn().mockResolvedValue({ ok: true });
        vi.mocked(deploymentFetch).mockReturnValue(mockFetch as any);

        await env.parseAsync(
          ["set", "ABC", "DEF", "--deployment", "myteam:myproject:prod"],
          { from: "user" },
        );

        expect(bigBrainAPIMaybeThrows).toHaveBeenCalledWith(
          expect.objectContaining({
            path: "deployment/provision_and_authorize",
            data: expect.objectContaining({
              teamSlug: "myteam",
              projectSlug: "myproject",
              deploymentType: "prod",
            }),
          }),
        );
        expect(deploymentFetch).toHaveBeenCalledWith(
          expect.anything(),
          expect.objectContaining({
            deploymentUrl: "https://fq-prod-deploy.convex.cloud",
            adminKey: "fq-prod-key",
          }),
        );
      });

      it("resolves --deployment team:project:dev to dev deployment in fully qualified team/project", async () => {
        setupBigBrainRoutes({
          "teams/myteam/projects/myproject/deployments": () => true,
          "deployment/authorize_within_current_project": () => ({
            adminKey: "fq-dev-key",
            url: "https://fq-dev-deploy.convex.cloud",
            deploymentName: "fq-dev-deploy",
            deploymentType: "dev",
          }),
        });
        mockPlatformGet.mockResolvedValue({
          data: { name: "fq-dev-deploy" },
          error: undefined,
        });

        const mockFetch = vi.fn().mockResolvedValue({ ok: true });
        vi.mocked(deploymentFetch).mockReturnValue(mockFetch as any);

        await env.parseAsync(
          ["set", "ABC", "DEF", "--deployment", "myteam:myproject:dev"],
          { from: "user" },
        );

        expect(mockPlatformGet).toHaveBeenCalledWith(
          "/teams/{team_id_or_slug}/projects/{project_slug}/deployment",
          expect.objectContaining({
            params: expect.objectContaining({
              path: {
                team_id_or_slug: "myteam",
                project_slug: "myproject",
              },
              query: { defaultDev: true },
            }),
          }),
        );
        expect(bigBrainAPI).toHaveBeenCalledWith(
          expect.objectContaining({
            path: "deployment/authorize_within_current_project",
            data: expect.objectContaining({
              selectedDeploymentName: "fq-dev-deploy",
            }),
          }),
        );
        expect(deploymentFetch).toHaveBeenCalledWith(
          expect.anything(),
          expect.objectContaining({
            deploymentUrl: "https://fq-dev-deploy.convex.cloud",
            adminKey: "fq-dev-key",
          }),
        );
      });

      it("errors when --deployment used with self-hosted deployment", async () => {
        delete process.env.CONVEX_DEPLOYMENT;
        process.env.CONVEX_SELF_HOSTED_URL = "http://localhost:3210";
        process.env.CONVEX_SELF_HOSTED_ADMIN_KEY = "self-hosted-key";

        await expect(
          env.parseAsync(["set", "ABC", "DEF", "--deployment", "prod"], {
            from: "user",
          }),
        ).rejects.toThrow();

        expect(deploymentFetch).not.toHaveBeenCalled();
      });

      it("errors when --deployment used with --url and --admin-key", async () => {
        await expect(
          env.parseAsync(
            [
              "set",
              "ABC",
              "DEF",
              "--deployment",
              "prod",
              "--url",
              "https://example.convex.cloud",
              "--admin-key",
              "mykey",
            ],
            { from: "user" },
          ),
        ).rejects.toThrow();
      });

      it("resolves --deployment local to local deployment credentials", async () => {
        vi.mocked(loadProjectLocalConfig).mockReturnValue({
          deploymentName: "local-my_team-my_project-abc",
          config: {
            ports: { cloud: 3210, site: 3211 },
            adminKey: "local-key",
            backendVersion: "1.0.0",
          },
        });
        vi.mocked(loadLocalDeploymentCredentials).mockResolvedValue({
          deploymentName: "local-my_team-my_project-abc",
          deploymentUrl: "http://127.0.0.1:3210",
          adminKey: "local-key",
        });
        // The local deployment name is resolved via Big Brain for project
        // access checks (checkAccessToSelectedProject)
        setupBigBrainRoutes({
          "deployment/local-my_team-my_project-abc/team_and_project": () => ({
            team: "my-team",
            project: "my-project",
            teamId: 1,
            projectId: 1,
          }),
        });

        const mockFetch = vi.fn().mockResolvedValue({ ok: true });
        vi.mocked(deploymentFetch).mockReturnValue(mockFetch as any);

        await env.parseAsync(["set", "ABC", "DEF", "--deployment", "local"], {
          from: "user",
        });

        expect(loadLocalDeploymentCredentials).toHaveBeenCalledWith(
          expect.anything(),
          "local-my_team-my_project-abc",
        );
        expect(deploymentFetch).toHaveBeenCalledWith(
          expect.anything(),
          expect.objectContaining({
            deploymentUrl: "http://127.0.0.1:3210",
            adminKey: "local-key",
          }),
        );
      });

      it("resolves --deployment local without CONVEX_DEPLOYMENT set", async () => {
        delete process.env.CONVEX_DEPLOYMENT;

        vi.mocked(loadProjectLocalConfig).mockReturnValue({
          deploymentName: "local-my_team-my_project-abc",
          config: {
            ports: { cloud: 3210, site: 3211 },
            adminKey: "local-key",
            backendVersion: "1.0.0",
          },
        });
        vi.mocked(loadLocalDeploymentCredentials).mockResolvedValue({
          deploymentName: "local-my_team-my_project-abc",
          deploymentUrl: "http://127.0.0.1:3210",
          adminKey: "local-key",
        });
        setupBigBrainRoutes({
          "deployment/local-my_team-my_project-abc/team_and_project": () => ({
            team: "my-team",
            project: "my-project",
            teamId: 1,
            projectId: 1,
          }),
        });

        const mockFetch = vi.fn().mockResolvedValue({ ok: true });
        vi.mocked(deploymentFetch).mockReturnValue(mockFetch as any);

        await env.parseAsync(["set", "ABC", "DEF", "--deployment", "local"], {
          from: "user",
        });

        expect(deploymentFetch).toHaveBeenCalledWith(
          expect.anything(),
          expect.objectContaining({
            deploymentUrl: "http://127.0.0.1:3210",
            adminKey: "local-key",
          }),
        );
      });

      it("errors when --deployment local but no local deployment exists", async () => {
        vi.mocked(loadProjectLocalConfig).mockReturnValue(null);

        await expect(
          env.parseAsync(["set", "ABC", "DEF", "--deployment", "local"], {
            from: "user",
          }),
        ).rejects.toThrow();

        expect(process.stderr.write).toHaveBeenCalledWith(
          expect.stringContaining("No local deployment found"),
        );
        expect(process.stderr.write).toHaveBeenCalledWith(
          expect.stringContaining("npx convex deployment create local"),
        );
      });
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

    it("dev with CONVEX_DEPLOYMENT=local:... uses fresh credentials from handleLocalDeployment", async () => {
      process.env.CONVEX_DEPLOYMENT = "local:my-local-deployment";
      vi.mocked(readGlobalConfig).mockReturnValue({
        accessToken: "test-token",
      });

      // loadLocalDeploymentCredentials returns stale saved config (e.g. from a
      // previous run on a different port).
      vi.mocked(loadLocalDeploymentCredentials).mockResolvedValue({
        deploymentName: "my-local-deployment",
        deploymentUrl: "http://127.0.0.1:3212",
        adminKey: "stale|admin|key",
      });

      // handleLocalDeployment starts a new backend, potentially on different
      // ports, and returns the actual credentials.
      vi.mocked(handleLocalDeployment).mockResolvedValue({
        deploymentName: "my-local-deployment",
        deploymentUrl: "http://127.0.0.1:3210",
        adminKey: "fresh|admin|key",
        onActivity: async () => {},
      } as any);

      setupBigBrainRoutes({
        "deployment/my-local-deployment/team_and_project": () => ({
          team: "my-team",
          project: "my-project",
          teamId: 1,
          projectId: 1,
        }),
      });

      await dev.parseAsync([], { from: "user" });

      expect(handleLocalDeployment).toHaveBeenCalledWith(
        expect.anything(),
        expect.objectContaining({
          teamSlug: "my-team",
          projectSlug: "my-project",
        }),
      );
      // Must use the fresh credentials from handleLocalDeployment, not the
      // stale ones from loadLocalDeploymentCredentials.
      expect(devAgainstDeployment).toHaveBeenCalledWith(
        expect.anything(),
        expect.objectContaining({
          url: "http://127.0.0.1:3210",
          adminKey: "fresh|admin|key",
        }),
        expect.anything(),
      );
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

    it("resolves CONVEX_DEPLOYMENT to the configured deployment", async () => {
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
        "deployment/authorize_within_current_project": () => ({
          adminKey: "dev-key",
          url: "https://joyful-capybara-123.convex.cloud",
          deploymentName: "joyful-capybara-123",
          deploymentType: "dev",
        }),
      });

      await dev.parseAsync([], { from: "user" });

      expect(bigBrainAPI).toHaveBeenCalledWith(
        expect.objectContaining({
          path: "deployment/authorize_within_current_project",
          data: expect.objectContaining({
            selectedDeploymentName: "joyful-capybara-123",
          }),
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

    it("resolves CONVEX_DEPLOYMENT with --cloud to the configured deployment", async () => {
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
        "deployment/authorize_within_current_project": () => ({
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

    describe("non-interactive terminal", () => {
      beforeEach(() => {
        process.stdin.isTTY = false as any;
      });

      it("non-interactive, not logged in, no config → uses anonymous deployment", async () => {
        await dev.parseAsync(["--once"], { from: "user" });

        expect(handleAnonymousDeployment).toHaveBeenCalled();
        expect(bigBrainAPI).not.toHaveBeenCalled();
        expect(bigBrainAPIMaybeThrows).not.toHaveBeenCalled();
        expect(validateOrSelectTeam).not.toHaveBeenCalled();
        expect(validateOrSelectProject).not.toHaveBeenCalled();
      });

      it("non-interactive, logged in, no CONVEX_DEPLOYMENT → uses anonymous deployment", async () => {
        vi.mocked(readGlobalConfig).mockReturnValue({
          accessToken: "test-token",
        });

        await dev.parseAsync(["--once"], { from: "user" });

        expect(handleAnonymousDeployment).toHaveBeenCalled();
        expect(validateOrSelectTeam).not.toHaveBeenCalled();
        expect(validateOrSelectProject).not.toHaveBeenCalled();
      });

      it("non-interactive with CONVEX_DEPLOYMENT → resolves to the configured deployment", async () => {
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
          "deployment/authorize_within_current_project": () => ({
            adminKey: "dev-key",
            url: "https://joyful-capybara-123.convex.cloud",
            deploymentName: "joyful-capybara-123",
            deploymentType: "dev",
          }),
        });

        await dev.parseAsync([], { from: "user" });

        expect(devAgainstDeployment).toHaveBeenCalledWith(
          expect.anything(),
          expect.objectContaining({
            url: "https://joyful-capybara-123.convex.cloud",
            adminKey: "dev-key",
          }),
          expect.anything(),
        );
        expect(handleAnonymousDeployment).not.toHaveBeenCalled();
      });

      it("non-interactive with CONVEX_DEPLOY_KEY → uses deploy key", async () => {
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

        expect(devAgainstDeployment).toHaveBeenCalledWith(
          expect.anything(),
          expect.objectContaining({
            url: "https://swift-squirrel-234.convex.cloud",
            adminKey: "dev-key",
          }),
          expect.anything(),
        );
        expect(handleAnonymousDeployment).not.toHaveBeenCalled();
      });

      it("non-interactive with CONVEX_SELF_HOSTED_URL → uses self-hosted", async () => {
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
        expect(handleAnonymousDeployment).not.toHaveBeenCalled();
      });
    });
  });

  it("dev --deployment crashes with helpful message pointing to deployment select", async () => {
    await expect(
      dev.parseAsync(["--deployment", "happy-animal-123"], { from: "user" }),
    ).rejects.toThrow();

    expect(process.stderr.write).toHaveBeenCalledWith(
      expect.stringContaining("npx convex deployment select happy-animal-123"),
    );
    expect(devAgainstDeployment).not.toHaveBeenCalled();
  });
});
