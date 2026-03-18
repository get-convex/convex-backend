import { describe, test, expect, vi, beforeEach, afterEach } from "vitest";
// @inquirer/testing/vitest must be imported before modules that use @inquirer/*
import { screen } from "@inquirer/testing/vitest";
import {
  typedPlatformClient,
  typedBigBrainClient,
  selectRegion,
  logNoDefaultRegionMessage,
} from "./lib/utils/utils.js";
import { PlatformProjectDetails } from "@convex-dev/platform/managementApi";
import {
  getDeploymentSelection,
  getProjectDetails,
} from "./lib/deploymentSelection.js";
import { selectDeployment } from "./deploymentSelect.js";
import { deploymentCreate, resolveRegionDetails } from "./deploymentCreate.js";

vi.mock("@sentry/node", () => ({
  captureException: vi.fn(),
  close: vi.fn(),
}));

vi.mock("./lib/utils/utils.js", () => ({
  typedPlatformClient: vi.fn(),
  typedBigBrainClient: vi.fn(),
  selectRegion: vi.fn(),
  logNoDefaultRegionMessage: vi.fn(),
}));

vi.mock("./lib/deploymentSelection.js", () => ({
  initializeBigBrainAuth: vi.fn(),
  getDeploymentSelection: vi.fn(),
  getProjectDetails: vi.fn(),
}));

vi.mock("./deploymentSelect.js", () => ({
  selectDeployment: vi.fn(),
}));

const mockRegions = [
  {
    name: "aws-us-east-1" as const,
    displayName: "US East (Virginia)",
    available: true,
  },
  {
    name: "aws-eu-west-1" as const,
    displayName: "EU West (Ireland)",
    available: true,
  },
];

const mockPlatformGet = vi.fn();
const mockPlatformPost = vi.fn();
const mockBigBrainGet = vi.fn();

function setupPlatformClient() {
  vi.mocked(typedPlatformClient).mockReturnValue({
    GET: mockPlatformGet,
    POST: mockPlatformPost,
  } as any);
  vi.mocked(typedBigBrainClient).mockReturnValue({
    GET: mockBigBrainGet,
  } as any);
}

// Suppress process.exit and stderr
beforeEach(() => {
  vi.spyOn(process, "exit").mockImplementation((() => {
    throw new Error("process.exit called");
  }) as any);
  vi.spyOn(process.stderr, "write").mockImplementation(() => true);
  mockPlatformGet.mockReset();
  mockPlatformPost.mockReset();
  mockBigBrainGet.mockReset();
});

afterEach(() => {
  vi.restoreAllMocks();
});

const fakeProject = {
  id: 123,
  teamId: 456,
  slug: "my-project",
  createTime: 0,
  name: "My Project",
  teamSlug: "my-team",
} satisfies PlatformProjectDetails;

const createdDeployment = {
  kind: "cloud" as const,
  reference: "dev/my-deployment",
  deploymentType: "dev" as const,
  isDefault: false,
};

function setupPlatformForCreate(overrides?: Record<string, unknown>) {
  setupPlatformClient();
  mockPlatformGet.mockResolvedValue({ data: { items: mockRegions } });
  mockPlatformPost.mockResolvedValue({
    data: { ...createdDeployment, ...overrides },
  });
}

describe("non-interactive create flow", () => {
  beforeEach(() => {
    vi.mocked(getDeploymentSelection).mockReset();
    vi.mocked(getProjectDetails).mockReset();
    vi.mocked(selectDeployment).mockReset();
  });

  describe("validation errors", () => {
    test("crashes when no ref and no --default", async () => {
      await expect(
        deploymentCreate.parseAsync(["--type", "dev"], { from: "user" }),
      ).rejects.toThrow();
      expect(process.stderr.write).toHaveBeenCalledWith(
        expect.stringContaining("Specify a deployment ref"),
      );
    });

    test("crashes when --type is missing", async () => {
      await expect(
        deploymentCreate.parseAsync(["my-deployment"], { from: "user" }),
      ).rejects.toThrow();
      expect(process.stderr.write).toHaveBeenCalledWith(
        expect.stringContaining("--type is required"),
      );
    });
  });

  describe("with project configured", () => {
    beforeEach(() => {
      vi.mocked(getDeploymentSelection).mockResolvedValue({
        kind: "existingDeployment",
        deploymentToActOn: {
          url: "https://joyful-capybara-123.convex.cloud",
          adminKey: "admin-key",
          deploymentFields: {
            deploymentName: "joyful-capybara-123",
            deploymentType: "dev",
            teamSlug: "my-team",
            projectSlug: "my-project",
          },
          source: "deployKey" as const,
        },
      });
      vi.mocked(getProjectDetails).mockResolvedValue(fakeProject);
    });

    test("creates a dev deployment with ref and --type dev", async () => {
      setupPlatformForCreate();

      await deploymentCreate.parseAsync(["my-deployment", "--type", "dev"], {
        from: "user",
      });

      expect(mockPlatformPost).toHaveBeenCalledWith(
        "/projects/{project_id}/create_deployment",
        expect.objectContaining({
          params: { path: { project_id: 123 } },
          body: {
            type: "dev",
            region: null,
            reference: "my-deployment",
            isDefault: null,
          },
        }),
      );
    });

    test("creates a prod deployment with ref and --type prod", async () => {
      setupPlatformForCreate({
        deploymentType: "prod",
        reference: "staging",
      });

      await deploymentCreate.parseAsync(["staging", "--type", "prod"], {
        from: "user",
      });

      expect(mockPlatformPost).toHaveBeenCalledWith(
        "/projects/{project_id}/create_deployment",
        expect.objectContaining({
          body: expect.objectContaining({
            type: "prod",
            reference: "staging",
          }),
        }),
      );
    });

    test("creates a deployment with --default flag", async () => {
      setupPlatformForCreate({ isDefault: true });

      await deploymentCreate.parseAsync(
        ["my-deployment", "--type", "dev", "--default"],
        { from: "user" },
      );

      expect(mockPlatformPost).toHaveBeenCalledWith(
        "/projects/{project_id}/create_deployment",
        expect.objectContaining({
          body: expect.objectContaining({
            isDefault: true,
          }),
        }),
      );
    });

    test("creates a default deployment without a ref", async () => {
      setupPlatformForCreate({ isDefault: true, reference: null });

      await deploymentCreate.parseAsync(["--type", "dev", "--default"], {
        from: "user",
      });

      expect(mockPlatformPost).toHaveBeenCalledWith(
        "/projects/{project_id}/create_deployment",
        expect.objectContaining({
          body: {
            type: "dev",
            region: null,
            reference: null,
            isDefault: true,
          },
        }),
      );
    });

    test("creates a deployment with --region full name", async () => {
      setupPlatformForCreate();

      await deploymentCreate.parseAsync(
        ["my-deployment", "--type", "dev", "--region", "aws-eu-west-1"],
        { from: "user" },
      );

      expect(mockPlatformGet).toHaveBeenCalledWith(
        "/teams/{team_id}/list_deployment_regions",
        expect.objectContaining({
          params: { path: { team_id: "456" } },
        }),
      );
      expect(mockPlatformPost).toHaveBeenCalledWith(
        "/projects/{project_id}/create_deployment",
        expect.objectContaining({
          body: expect.objectContaining({
            region: "aws-eu-west-1",
          }),
        }),
      );
    });

    test("creates a deployment with --region alias", async () => {
      setupPlatformForCreate();

      await deploymentCreate.parseAsync(
        ["my-deployment", "--type", "dev", "--region", "us"],
        { from: "user" },
      );

      expect(mockPlatformPost).toHaveBeenCalledWith(
        "/projects/{project_id}/create_deployment",
        expect.objectContaining({
          body: expect.objectContaining({
            region: "aws-us-east-1",
          }),
        }),
      );
    });

    test("fails with invalid --region", async () => {
      setupPlatformForCreate();

      await expect(
        deploymentCreate.parseAsync(
          ["my-deployment", "--type", "dev", "--region", "invalid-region"],
          { from: "user" },
        ),
      ).rejects.toThrow();
      expect(process.stderr.write).toHaveBeenCalledWith(
        expect.stringContaining('Invalid region "invalid-region"'),
      );
    });

    test("creates a deployment with --select calls selectDeployment", async () => {
      setupPlatformForCreate({
        reference: "dev/my-deployment",
      });

      await deploymentCreate.parseAsync(
        ["my-deployment", "--type", "dev", "--select"],
        { from: "user" },
      );

      expect(selectDeployment).toHaveBeenCalledWith(
        expect.anything(),
        "dev/my-deployment",
      );
    });
  });

  describe("with team:project:ref syntax", () => {
    test("uses getProjectDetails with teamAndProjectSlugs", async () => {
      vi.mocked(getProjectDetails).mockResolvedValue({
        ...fakeProject,
        slug: "other-project",
        teamSlug: "other-team",
      });
      setupPlatformForCreate();

      await deploymentCreate.parseAsync(
        ["other-team:other-project:my-deployment", "--type", "dev"],
        { from: "user" },
      );

      expect(getProjectDetails).toHaveBeenCalledWith(expect.anything(), {
        kind: "teamAndProjectSlugs",
        teamSlug: "other-team",
        projectSlug: "other-project",
      });
      expect(getDeploymentSelection).not.toHaveBeenCalled();
      expect(mockPlatformPost).toHaveBeenCalledWith(
        "/projects/{project_id}/create_deployment",
        expect.objectContaining({
          body: expect.objectContaining({
            reference: "my-deployment",
          }),
        }),
      );
    });
  });

  describe("without project configured", () => {
    beforeEach(() => {
      vi.mocked(getDeploymentSelection).mockResolvedValue({
        kind: "chooseProject",
      } as any);
    });

    test("fails with bare ref when no project context", async () => {
      await expect(
        deploymentCreate.parseAsync(["my-deployment", "--type", "dev"], {
          from: "user",
        }),
      ).rejects.toThrow();
      expect(process.stderr.write).toHaveBeenCalledWith(
        expect.stringContaining("No project configured yet"),
      );
    });

    test("succeeds with team:project:ref syntax", async () => {
      vi.mocked(getProjectDetails).mockResolvedValue(fakeProject);
      setupPlatformForCreate();

      await deploymentCreate.parseAsync(
        ["my-team:my-project:my-deployment", "--type", "dev"],
        { from: "user" },
      );

      expect(getProjectDetails).toHaveBeenCalledWith(expect.anything(), {
        kind: "teamAndProjectSlugs",
        teamSlug: "my-team",
        projectSlug: "my-project",
      });
      expect(mockPlatformPost).toHaveBeenCalled();
    });
  });
});

describe("interactive create flow", () => {
  beforeEach(() => {
    process.stdin.isTTY = true;
    vi.mocked(getDeploymentSelection).mockReset();
    vi.mocked(getProjectDetails).mockReset();
    vi.mocked(selectDeployment).mockReset();
    vi.mocked(selectRegion).mockReset();

    // Default: project configured via deployment selection
    vi.mocked(getDeploymentSelection).mockResolvedValue({
      kind: "existingDeployment",
      deploymentToActOn: {
        url: "https://joyful-capybara-123.convex.cloud",
        adminKey: "admin-key",
        deploymentFields: {
          deploymentName: "joyful-capybara-123",
          deploymentType: "dev",
          teamSlug: "my-team",
          projectSlug: "my-project",
        },
        source: "deployKey" as const,
      },
    });
    vi.mocked(getProjectDetails).mockResolvedValue(fakeProject);
  });

  afterEach(() => {
    process.stdin.isTTY = false;
  });

  function setupPlatformRoutes(routes: Record<string, (args: any) => any>) {
    setupPlatformClient();
    mockPlatformGet.mockImplementation((path: string, args: any) => {
      for (const [routePath, handler] of Object.entries(routes)) {
        if (path === routePath || path.startsWith(routePath)) {
          return { data: handler(args) };
        }
      }
      throw new Error(`Unmocked GET route: ${path}`);
    });
    mockPlatformPost.mockResolvedValue({
      data: { ...createdDeployment },
    });
  }

  function setupDefaultRoutes() {
    setupPlatformRoutes({
      "/teams/{team_id}/list_deployment_regions": () => ({
        items: mockRegions,
      }),
    });
    mockBigBrainGet.mockResolvedValue({
      data: [{ id: 456, slug: "my-team", defaultRegion: "aws-us-east-1" }],
    });
  }

  test.each([
    { deploymentType: "dev" as const, downPresses: 0 },
    { deploymentType: "preview" as const, downPresses: 1 },
    { deploymentType: "prod" as const, downPresses: 2 },
  ])(
    "selecting $deploymentType calls endpoint with type=$deploymentType",
    async ({ deploymentType, downPresses }) => {
      setupDefaultRoutes();

      const promise = deploymentCreate.parseAsync([], { from: "user" });

      // Type prompt (select)
      await screen.next();
      expect(screen.getScreen()).toContain("Deployment type?");
      for (let i = 0; i < downPresses; i++) {
        screen.keypress("down");
      }
      screen.keypress("enter");

      // Ref prompt (input)
      await screen.next();
      expect(screen.getScreen()).toContain("Deployment ref?");
      screen.type("my-feature");
      screen.keypress("enter");

      await promise;

      expect(mockPlatformPost).toHaveBeenCalledWith(
        "/projects/{project_id}/create_deployment",
        expect.objectContaining({
          body: expect.objectContaining({
            type: deploymentType,
          }),
        }),
      );
    },
  );

  test("full prompt flow: select type, enter ref", async () => {
    setupDefaultRoutes();

    const promise = deploymentCreate.parseAsync([], { from: "user" });

    // Type prompt (select) — "dev" is first choice, just press enter
    await screen.next();
    expect(screen.getScreen()).toContain("Deployment type?");
    screen.keypress("enter");

    // Ref prompt (input)
    await screen.next();
    expect(screen.getScreen()).toContain("Deployment ref?");
    screen.type("my-feature");
    screen.keypress("enter");

    await promise;

    expect(mockPlatformPost).toHaveBeenCalledWith(
      "/projects/{project_id}/create_deployment",
      expect.objectContaining({
        body: {
          type: "dev",
          region: "aws-us-east-1",
          reference: "my-feature",
          isDefault: null,
        },
      }),
    );
  });

  test("partial flags: --type dev and ref provided, no prompts needed", async () => {
    setupDefaultRoutes();

    const promise = deploymentCreate.parseAsync(
      ["my-feature", "--type", "dev"],
      { from: "user" },
    );

    await promise;

    expect(mockPlatformPost).toHaveBeenCalledWith(
      "/projects/{project_id}/create_deployment",
      expect.objectContaining({
        body: expect.objectContaining({
          type: "dev",
          reference: "my-feature",
          isDefault: null,
        }),
      }),
    );
  });

  test("invalid ref retry: user enters invalid ref, then valid ref", async () => {
    setupDefaultRoutes();

    const promise = deploymentCreate.parseAsync(["--type", "dev"], {
      from: "user",
    });

    // Ref prompt — enter invalid ref "dev"
    await screen.next();
    expect(screen.getScreen()).toContain("Deployment ref?");
    screen.type("dev");
    screen.keypress("enter");

    // Re-prompted for ref after error
    await screen.next();
    expect(screen.getScreen()).toContain("Deployment ref?");
    screen.type("my-feature");
    screen.keypress("enter");

    await promise;

    expect(mockPlatformPost).toHaveBeenCalledWith(
      "/projects/{project_id}/create_deployment",
      expect.objectContaining({
        body: expect.objectContaining({
          reference: "my-feature",
          isDefault: null,
        }),
      }),
    );
  });

  test("--region invalid crashes", async () => {
    setupDefaultRoutes();

    await expect(
      deploymentCreate.parseAsync(
        ["my-feature", "--type", "dev", "--region", "invalid-region"],
        { from: "user" },
      ),
    ).rejects.toThrow();
    expect(process.stderr.write).toHaveBeenCalledWith(
      expect.stringContaining('Invalid region "invalid-region"'),
    );
  });

  test("no team default region: falls through to selectRegion", async () => {
    setupDefaultRoutes();
    // Override BigBrain to return team without defaultRegion
    mockBigBrainGet.mockResolvedValue({
      data: [{ id: 456, slug: "my-team" }],
    });
    vi.mocked(selectRegion).mockResolvedValue("aws-us-east-1");

    await deploymentCreate.parseAsync(["my-feature", "--type", "dev"], {
      from: "user",
    });

    expect(selectRegion).toHaveBeenCalledWith(expect.anything(), 456, "dev");
    expect(logNoDefaultRegionMessage).toHaveBeenCalledWith("my-team");
  });

  test("uses team default region when no --region provided", async () => {
    setupDefaultRoutes();

    const promise = deploymentCreate.parseAsync(
      ["my-feature", "--type", "dev"],
      { from: "user" },
    );

    await promise;

    expect(selectRegion).not.toHaveBeenCalled();
    expect(mockPlatformPost).toHaveBeenCalledWith(
      "/projects/{project_id}/create_deployment",
      expect.objectContaining({
        body: expect.objectContaining({
          region: "aws-us-east-1",
        }),
      }),
    );
  });
});

const availableRegions = mockRegions.filter((r) => r.available);

describe("resolveRegionDetails", () => {
  test("resolves region by alias", () => {
    const result = resolveRegionDetails(availableRegions, "us");
    expect(result).not.toBeNull();
    expect(result!.name).toBe("aws-us-east-1");
    expect(result!.displayName).toBe("US East (Virginia)");
  });

  test("resolves region by full name", () => {
    const result = resolveRegionDetails(availableRegions, "aws-eu-west-1");
    expect(result).not.toBeNull();
    expect(result!.name).toBe("aws-eu-west-1");
    expect(result!.displayName).toBe("EU West (Ireland)");
  });

  test("returns null on unknown region", () => {
    const result = resolveRegionDetails(availableRegions, "invalid-region");
    expect(result).toBeNull();
  });

  test("returns null on unavailable region", () => {
    const result = resolveRegionDetails(availableRegions, "aws-ap-southeast-1");
    expect(result).toBeNull();
  });
});
