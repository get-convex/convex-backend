import { RequestContext } from "./requestContext.js";
import { fetchDeploymentCredentialsProvisionProd } from "../api.js";
import path from "node:path";
import { deploymentSelectionFromOptions } from "../api.js";
import { z } from "zod";
import { ConvexTool } from "./tool.js";
import { deploymentDashboardUrlPage } from "../../dashboard.js";

const inputSchema = z.object({});
const outputSchema = z.object({
  url: z.string(),
  localPath: z.string(),
  cloud: z
    .object({
      deploymentName: z.string(),
      deploymentType: z.string(),
      dashboardUrl: z.string(),
    })
    .optional(),
});

const description = `
Get the status of the Convex deployment currently connected to the project.

Returns the URL of the deployment and the local path to the project.

If the deployment is provisioned on Convex Cloud, the deployment name, type,
and a link to the dashboard will be returned as well.
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
    let cloud;
    if (credentials.deploymentName && credentials.deploymentType) {
      const loginUrl = deploymentDashboardUrlPage(
        credentials.deploymentName,
        "",
      );
      cloud = {
        deploymentName: credentials.deploymentName,
        deploymentType: credentials.deploymentType,
        dashboardUrl: loginUrl,
      };
    }
    return {
      url: credentials.url,
      localPath: cwd,
      cloud,
    };
  },
};
