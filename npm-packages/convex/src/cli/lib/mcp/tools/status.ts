import { RequestContext } from "../requestContext.js";
import {
  DeploymentSelection,
  fetchDeploymentCredentialsProvisionProd,
} from "../../api.js";
import path from "node:path";
import { deploymentSelectionFromOptions } from "../../api.js";
import { z } from "zod";
import { ConvexTool } from "./index.js";
import { deploymentDashboardUrlPage } from "../../../dashboard.js";
import { encodeDeploymentSelector } from "../deploymentSelector.js";

const inputSchema = z.object({});
const outputSchema = z.object({
  projectDirectory: z.string(),
  availableDeployments: z.array(
    z.object({
      kind: z.string(),
      deploymentSelector: z.string(),
      url: z.string(),
      dashboardUrl: z.string().optional(),
    }),
  ),
});

const description = `
Get all available deployments for the currently configured Convex project.

Use this tool to find the deployment selector, URL, and dashboard URL for each
deployment associated with the project. Pass the deployment selector to other
tools to target a specific deployment.

When deployed to Convex Cloud, projects have a development ({"kind": "ownDev"}) and
production ({"kind": "ownProd"}) deployment. Generally default to using the development
deployment unless you'd specifically like to debug issues in production.

When running locally, there will be a single "urlWithAdminKey" deployment.
`.trim();

export const StatusTool: ConvexTool<typeof inputSchema, typeof outputSchema> = {
  name: "status",
  description,
  inputSchema,
  outputSchema,
  handler: async (ctx: RequestContext) => {
    const cwd = path.resolve(process.cwd());
    const deployment = await deploymentSelectionFromOptions(
      ctx,
      ctx.cmdOptions,
    );
    const credentials = await fetchDeploymentCredentialsProvisionProd(
      ctx,
      deployment,
    );
    const availableDeployments = [
      {
        kind: deployment.kind,
        deploymentSelector: encodeDeploymentSelector(deployment),
        url: credentials.url,
        dashboardUrl:
          credentials.deploymentName &&
          deploymentDashboardUrlPage(credentials.deploymentName, ""),
      },
    ];
    if (deployment.kind === "ownDev") {
      const prodDeployment: DeploymentSelection = { kind: "ownProd" };
      const prodCredentials = await fetchDeploymentCredentialsProvisionProd(
        ctx,
        prodDeployment,
      );
      if (prodCredentials.deploymentName && prodCredentials.deploymentType) {
        availableDeployments.push({
          kind: prodDeployment.kind,
          deploymentSelector: encodeDeploymentSelector(prodDeployment),
          url: prodCredentials.url,
          dashboardUrl: deploymentDashboardUrlPage(
            prodCredentials.deploymentName,
            "",
          ),
        });
      }
    }
    return {
      projectDirectory: cwd,
      availableDeployments,
    };
  },
};
