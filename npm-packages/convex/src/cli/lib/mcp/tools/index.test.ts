import { beforeEach, describe, expect, test, vi } from "vitest";
import {
  DeploymentSelectionWithinProject,
  loadSelectedDeploymentCredentials,
} from "../../api.js";
import { getDeploymentSelection } from "../../deploymentSelection.js";
import {
  encodeDeploymentSelector,
  McpOptions,
  RequestContext,
} from "../requestContext.js";
import { convexTools } from "./index.js";

vi.mock("../../api.js", async (importOriginal) => ({
  ...(await importOriginal<typeof import("../../api.js")>()),
  loadSelectedDeploymentCredentials: vi.fn(),
}));

vi.mock("../../deploymentSelection.js", async (importOriginal) => ({
  ...(await importOriginal<typeof import("../../deploymentSelection.js")>()),
  getDeploymentSelection: vi.fn(),
}));

const PROD_DEPLOYMENT_NAME = "happy-otter-123";
const projectDir = process.cwd();

// Selectors that resolve to the production deployment without being of kind
// `prod`.
const productionSelectors: DeploymentSelectionWithinProject[] = [
  { kind: "deploymentName", deploymentName: PROD_DEPLOYMENT_NAME },
  { kind: "deploymentSelector", selector: "prod" },
  { kind: "deploymentSelector", selector: "team:project:prod" },
];

const mutatingToolArgs: Record<string, Record<string, unknown>> = {
  run: { functionName: "messages:send", args: "{}" },
  envList: {},
  envGet: { name: "SECRET" },
  envSet: { name: "SECRET", value: "value" },
  envRemove: { name: "SECRET" },
};

const piiToolArgs: Record<string, Record<string, unknown>> = {
  data: { tableName: "messages", order: "asc" },
  logs: {},
  runOneoffQuery: { query: "export default query({ handler: () => null })" },
};

function getTool(name: string) {
  const tool = convexTools.find((t) => t.name === name);
  if (tool === undefined) {
    throw new Error(`Tool ${name} not found`);
  }
  return tool;
}

async function callTool(
  options: McpOptions,
  name: string,
  args: Record<string, unknown>,
) {
  const tool = getTool(name);
  return await tool.handler(
    new RequestContext(options),
    tool.inputSchema.parse(args),
  );
}

beforeEach(() => {
  vi.mocked(getDeploymentSelection).mockResolvedValue({
    kind: "deploymentWithinProject",
    targetProject: {
      kind: "teamAndProjectSlugs",
      teamSlug: "team",
      projectSlug: "project",
    },
    selectionWithinProject: {
      kind: "deploymentName",
      deploymentName: PROD_DEPLOYMENT_NAME,
    },
  });
  vi.mocked(loadSelectedDeploymentCredentials).mockResolvedValue({
    adminKey: "prod-admin-key",
    // Nothing should reach the deployment.
    url: "http://127.0.0.1:1",
    deploymentFields: {
      deploymentName: PROD_DEPLOYMENT_NAME,
      deploymentType: "prod",
      projectSlug: "project",
      teamSlug: "team",
      reference: "production",
      isDefault: true,
    },
  });
});

describe.each(productionSelectors)(
  "production deployment selected with %o",
  (deployment) => {
    const deploymentSelector = encodeDeploymentSelector(projectDir, deployment);

    test.each(Object.entries(mutatingToolArgs))(
      "%s is rejected",
      async (name, args) => {
        await expect(
          callTool({}, name, { deploymentSelector, ...args }),
        ).rejects.toMatchObject({
          printedMessage: expect.stringContaining(
            "This tool cannot be used with production deployments.",
          ),
        });
      },
    );

    test.each(Object.entries(piiToolArgs))(
      "%s is rejected",
      async (name, args) => {
        await expect(
          callTool({}, name, { deploymentSelector, ...args }),
        ).rejects.toMatchObject({
          printedMessage: expect.stringContaining(
            "This read-only tool may expose PII from production.",
          ),
        });
      },
    );
  },
);

describe("status", () => {
  test("marks a production deployment selected by name as read-only", async () => {
    const result = await callTool(
      { deploymentName: PROD_DEPLOYMENT_NAME },
      "status",
      { projectDir },
    );
    expect(result.availableDeployments).toEqual([
      expect.objectContaining({ kind: "deploymentName", readOnly: true }),
    ]);
  });

  test("marks it as readable with --cautiously-allow-production-pii", async () => {
    const result = await callTool(
      {
        deploymentName: PROD_DEPLOYMENT_NAME,
        cautiouslyAllowProductionPii: true,
      },
      "status",
      { projectDir },
    );
    expect(result.availableDeployments).toEqual([
      expect.objectContaining({ kind: "deploymentName", readOnly: false }),
    ]);
  });

  test("doesn't mark it with --dangerously-enable-production-deployments", async () => {
    const result = await callTool(
      {
        deploymentName: PROD_DEPLOYMENT_NAME,
        dangerouslyEnableProductionDeployments: true,
      },
      "status",
      { projectDir },
    );
    expect(result.availableDeployments).toHaveLength(1);
    expect(result.availableDeployments[0]).not.toHaveProperty("readOnly");
  });
});
