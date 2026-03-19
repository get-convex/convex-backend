import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import path from "path";
import { nodeFs } from "../bundler/fs.js";
import { deploymentSelect } from "./deploymentSelect.js";
import { bigBrainAPI, bigBrainAPIMaybeThrows } from "./lib/utils/utils.js";
import { runSystemQuery } from "./lib/run.js";
import { globalConfigPath } from "./lib/utils/globalConfig.js";

// Mock typedPlatformClient GET function — can be configured per test
const mockPlatformGet = vi.fn();

// In-memory filesystem — populated in beforeEach, written to by real configure code
let testFiles: Map<string, string>;

vi.mock("../bundler/fs.js", async (importOriginal) => {
  const actual = await importOriginal<typeof import("../bundler/fs.js")>();
  return {
    ...actual,
    nodeFs: {
      ...actual.nodeFs,
      exists: vi.fn(),
      readUtf8File: vi.fn(),
      writeUtf8File: vi.fn(),
      mkdir: vi.fn(),
    },
  };
});

vi.mock("./lib/utils/utils.js", async (importOriginal) => {
  const actual = await importOriginal<typeof import("./lib/utils/utils.js")>();
  return {
    ...actual,
    bigBrainAPI: vi.fn(),
    bigBrainAPIMaybeThrows: vi.fn(),
    typedPlatformClient: vi.fn(() => ({ GET: mockPlatformGet })),
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

vi.mock("./lib/run.js", () => ({
  runSystemQuery: vi.fn(),
}));

/**
 * Routes mock Big Brain API calls by path.
 * Both `bigBrainAPI` and `bigBrainAPIMaybeThrows` delegate to this.
 */
function setupBigBrainRoutes(routes: Record<string, (data?: any) => any>) {
  const handler = (args: { path: string; data?: any }) => {
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

describe("npx convex select", () => {
  let savedEnv: NodeJS.ProcessEnv;

  beforeEach(() => {
    savedEnv = { ...process.env };
    process.env = {};

    // Start with minimal filesystem: package.json for readProjectConfig fallback
    testFiles = new Map([[path.resolve("package.json"), "{}"]]);

    vi.resetAllMocks();

    // Wire up the in-memory filesystem to the nodeFs mock
    vi.mocked(nodeFs.exists).mockImplementation((p: string) =>
      testFiles.has(path.resolve(p)),
    );
    vi.mocked(nodeFs.readUtf8File).mockImplementation((p: string) => {
      const content = testFiles.get(path.resolve(p));
      if (content === undefined) {
        const err: any = new Error(
          `ENOENT: no such file or directory, open '${p}'`,
        );
        err.code = "ENOENT";
        throw err;
      }
      return content;
    });
    vi.mocked(nodeFs.writeUtf8File).mockImplementation(
      (p: string, content: string) => {
        testFiles.set(path.resolve(p), content);
      },
    );

    // runSystemQuery is called by fetchDeploymentCanonicalSiteUrl to look up CONVEX_SITE_URL
    vi.mocked(runSystemQuery).mockResolvedValue({
      name: "CONVEX_SITE_URL",
      value: "https://example.convex.site",
    });

    // typedPlatformClient is used for reference-based deployment resolution
    vi.mocked(mockPlatformGet).mockResolvedValue({ data: undefined });
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

  describe("with project configured", () => {
    beforeEach(() => {
      process.env.CONVEX_DEPLOYMENT = "dev:joyful-capybara-123";
      testFiles.set(
        globalConfigPath(),
        JSON.stringify({ accessToken: "test-token" }),
      );
    });

    it("selects a dev deployment by name (abc-xyz-123)", async () => {
      // For a deployment name selector, the system looks up the *selected*
      // deployment's team/project (not the current CONVEX_DEPLOYMENT's).
      setupBigBrainRoutes({
        "deployment/clever-otter-890/team_and_project": () => ({
          team: "my-team",
          project: "my-project",
          teamId: 1,
          projectId: 1,
        }),
        "deployment/authorize_within_current_project": () => ({
          adminKey: "dev-key",
          url: "https://clever-otter-890.convex.cloud",
          deploymentName: "clever-otter-890",
          deploymentType: "dev",
        }),
      });

      await deploymentSelect.parseAsync(["clever-otter-890"], { from: "user" });

      expect(bigBrainAPI).toHaveBeenCalledWith(
        expect.objectContaining({
          path: "deployment/authorize_within_current_project",
          data: expect.objectContaining({
            selectedDeploymentName: "clever-otter-890",
          }),
        }),
      );
      const envContent = testFiles.get(path.resolve(".env.local"))!;
      expect(envContent).toContain("CONVEX_DEPLOYMENT=dev:clever-otter-890");
      expect(envContent).toContain("team: my-team, project: my-project");
      expect(envContent).toContain(
        "CONVEX_URL=https://clever-otter-890.convex.cloud",
      );
    });

    it("selects dev deployment with 'dev' selector", async () => {
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

      await deploymentSelect.parseAsync(["dev"], { from: "user" });

      expect(bigBrainAPIMaybeThrows).toHaveBeenCalledWith(
        expect.objectContaining({
          path: "deployment/provision_and_authorize",
          data: expect.objectContaining({ deploymentType: "dev" }),
        }),
      );
      const envContent = testFiles.get(path.resolve(".env.local"))!;
      expect(envContent).toContain("CONVEX_DEPLOYMENT=dev:joyful-capybara-123");
    });

    it("selects dev deployment by reference 'dev/nicolas'", async () => {
      setupBigBrainRoutes({
        "deployment/joyful-capybara-123/team_and_project": () => ({
          team: "my-team",
          project: "my-project",
          teamId: 1,
          projectId: 1,
        }),
        "teams/my-team/projects/my-project/deployments": () => true,
        "deployment/authorize_within_current_project": () => ({
          adminKey: "nicolas-key",
          url: "https://nicolas-dev-123.convex.cloud",
          deploymentName: "nicolas-dev-123",
          deploymentType: "dev",
        }),
      });
      mockPlatformGet.mockResolvedValue({
        data: { name: "nicolas-dev-123" },
        error: undefined,
      });

      await deploymentSelect.parseAsync(["dev/nicolas"], { from: "user" });

      expect(mockPlatformGet).toHaveBeenCalledWith(
        "/teams/{team_id_or_slug}/projects/{project_slug}/deployment",
        expect.objectContaining({
          params: expect.objectContaining({
            path: { team_id_or_slug: "my-team", project_slug: "my-project" },
            query: { reference: "dev/nicolas" },
          }),
        }),
      );
      expect(bigBrainAPI).toHaveBeenCalledWith(
        expect.objectContaining({
          path: "deployment/authorize_within_current_project",
          data: expect.objectContaining({
            selectedDeploymentName: "nicolas-dev-123",
          }),
        }),
      );
      expect(testFiles.has(path.resolve(".env.local"))).toBe(true);
    });

    it("selects a preview deployment in another project 'other-project:preview/my-feature'", async () => {
      setupBigBrainRoutes({
        "deployment/joyful-capybara-123/team_and_project": () => ({
          team: "my-team",
          project: "my-project",
          teamId: 1,
          projectId: 1,
        }),
        "teams/my-team/projects/other-project/deployments": () => true,
        "deployment/authorize_within_current_project": () => ({
          adminKey: "preview-key",
          url: "https://feature-preview-123.convex.cloud",
          deploymentName: "feature-preview-123",
          deploymentType: "preview",
        }),
      });
      mockPlatformGet.mockResolvedValue({
        data: { name: "feature-preview-123" },
        error: undefined,
      });

      await deploymentSelect.parseAsync(["other-project:preview/my-feature"], {
        from: "user",
      });

      expect(mockPlatformGet).toHaveBeenCalledWith(
        "/teams/{team_id_or_slug}/projects/{project_slug}/deployment",
        expect.objectContaining({
          params: expect.objectContaining({
            path: {
              team_id_or_slug: "my-team",
              project_slug: "other-project",
            },
            query: { reference: "preview/my-feature" },
          }),
        }),
      );
      const envContent = testFiles.get(path.resolve(".env.local"))!;
      expect(envContent).toContain(
        "CONVEX_DEPLOYMENT=preview:feature-preview-123",
      );
    });

    describe("prod deployment restrictions", () => {
      it("fails with an error message when 'prod' selector is used", async () => {
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

        await expect(
          deploymentSelect.parseAsync(["prod"], { from: "user" }),
        ).rejects.toThrow();

        expect(process.stderr.write).toHaveBeenCalledWith(
          expect.stringContaining("--deployment prod"),
        );
        expect(testFiles.has(path.resolve(".env.local"))).toBe(false);
      });

      it("fails with an error message when a deployment name resolves to a prod deployment", async () => {
        setupBigBrainRoutes({
          "deployment/graceful-puffin-456/team_and_project": () => ({
            team: "my-team",
            project: "my-project",
            teamId: 1,
            projectId: 1,
          }),
          "deployment/authorize_within_current_project": () => ({
            adminKey: "prod-key",
            url: "https://graceful-puffin-456.convex.cloud",
            deploymentName: "graceful-puffin-456",
            deploymentType: "prod",
          }),
        });

        await expect(
          deploymentSelect.parseAsync(["graceful-puffin-456"], {
            from: "user",
          }),
        ).rejects.toThrow();

        expect(process.stderr.write).toHaveBeenCalledWith(
          expect.stringContaining("--deployment graceful-puffin-456"),
        );
        expect(testFiles.has(path.resolve(".env.local"))).toBe(false);
      });
    });

    describe("side effects on successful selection", () => {
      it("fetches the site URL using the resolved deployment credentials", async () => {
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

        await deploymentSelect.parseAsync(["dev"], { from: "user" });

        expect(runSystemQuery).toHaveBeenCalledWith(
          expect.anything(),
          expect.objectContaining({
            adminKey: "dev-key",
            deploymentUrl: "https://joyful-capybara-123.convex.cloud",
          }),
        );
      });

      it("writes the fetched site URL to the env file", async () => {
        vi.mocked(runSystemQuery).mockResolvedValue({
          name: "CONVEX_SITE_URL",
          value: "https://joyful-capybara-123.convex.site",
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

        await deploymentSelect.parseAsync(["dev"], { from: "user" });

        const envContent = testFiles.get(path.resolve(".env.local"))!;
        expect(envContent).toContain(
          "CONVEX_SITE_URL=https://joyful-capybara-123.convex.site",
        );
      });

      it("uses the existing deployment name to detect unchanged selections", async () => {
        // deploymentNameFromSelection(currentSelection) extracts "joyful-capybara-123"
        // from process.env.CONVEX_DEPLOYMENT ("dev:joyful-capybara-123") and passes
        // it as existingValue to configure so it can detect whether the selection changed.
        // Here we verify the full chain ran: the correct name is written to .env.local.
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

        await deploymentSelect.parseAsync(["dev"], { from: "user" });

        const envContent = testFiles.get(path.resolve(".env.local"))!;
        expect(envContent).toContain(
          "CONVEX_DEPLOYMENT=dev:joyful-capybara-123",
        );
      });
    });
  });

  describe("without project configured", () => {
    beforeEach(() => {
      delete process.env.CONVEX_DEPLOYMENT;
      testFiles.set(
        globalConfigPath(),
        JSON.stringify({ accessToken: "test-token" }),
      );
    });

    it("fails with 'No project configured' for the 'dev' selector", async () => {
      await expect(
        deploymentSelect.parseAsync(["dev"], { from: "user" }),
      ).rejects.toThrow();

      expect(process.stderr.write).toHaveBeenCalledWith(
        expect.stringContaining("No project configured"),
      );
    });

    it("fails with 'No project configured' for a simple reference selector", async () => {
      await expect(
        deploymentSelect.parseAsync(["staging"], { from: "user" }),
      ).rejects.toThrow();

      expect(process.stderr.write).toHaveBeenCalledWith(
        expect.stringContaining("No project configured"),
      );
    });

    it("fails with 'No project configured' for a project:reference selector (needs team context)", async () => {
      await expect(
        deploymentSelect.parseAsync(["my-project:staging"], { from: "user" }),
      ).rejects.toThrow();

      expect(process.stderr.write).toHaveBeenCalledWith(
        expect.stringContaining("No project configured"),
      );
    });

    it("succeeds with a fully-qualified 'team:project:ref' selector", async () => {
      setupBigBrainRoutes({
        "deployment/authorize_within_current_project": () => ({
          adminKey: "fq-key",
          url: "https://fully-qualified-123.convex.cloud",
          deploymentName: "fully-qualified-123",
          deploymentType: "dev",
        }),
        "teams/my-team/projects/my-project/deployments": () => true,
      });
      mockPlatformGet.mockResolvedValue({
        data: { name: "fully-qualified-123" },
        error: undefined,
      });

      await deploymentSelect.parseAsync(["my-team:my-project:staging"], {
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
      const envContent = testFiles.get(path.resolve(".env.local"))!;
      expect(envContent).toContain("CONVEX_DEPLOYMENT=dev:fully-qualified-123");
    });

    it("succeeds with a deployment name directly (does not need project context)", async () => {
      // Deployment names (abc-xyz-123 pattern) don't require a project to
      // already be configured — they look up their own team/project info.
      setupBigBrainRoutes({
        "deployment/clever-otter-890/team_and_project": () => ({
          team: "my-team",
          project: "my-project",
          teamId: 1,
          projectId: 1,
        }),
        "deployment/authorize_within_current_project": () => ({
          adminKey: "dev-key",
          url: "https://clever-otter-890.convex.cloud",
          deploymentName: "clever-otter-890",
          deploymentType: "dev",
        }),
      });

      await deploymentSelect.parseAsync(["clever-otter-890"], { from: "user" });

      // deploymentNameFromSelection(currentSelection) returns null when there
      // is no CONVEX_DEPLOYMENT configured (kind === "chooseProject"), meaning
      // configure treats this as a brand-new selection.
      const envContent = testFiles.get(path.resolve(".env.local"))!;
      expect(envContent).toContain("CONVEX_DEPLOYMENT=dev:clever-otter-890");
    });
  });
});
