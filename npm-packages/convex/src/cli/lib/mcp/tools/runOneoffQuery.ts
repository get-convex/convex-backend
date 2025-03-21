import { z } from "zod";
import { ConvexTool } from "./index.js";
import { loadSelectedDeploymentCredentials } from "../../api.js";
import { getDeploymentSelection } from "../../deploymentSelection.js";

const inputSchema = z.object({
  deploymentSelector: z
    .string()
    .describe(
      "Deployment selector (from the status tool) to run the query on.",
    ),
  query: z
    .string()
    .describe(
      "The query to run. This should be valid JavaScript code that returns a value.",
    ),
});

const outputSchema = z.object({
  result: z.any().describe("The result returned by the query"),
  logLines: z
    .array(z.string())
    .describe("The log lines generated by the query"),
});

const description = `
Run a one-off readonly query on your Convex deployment.

This tool executes a JavaScript string as a query in your Convex deployment.
The query should follow Convex guidelines and use the following setup:

\`\`\`js
import { query, internalQuery } from "convex:/_system/repl/wrappers.js";

export default query({
  handler: async (ctx) => {
    console.log("Write and test your query function here!");
  },
});
\`\`\`

Note that there are no imports available in this environment. The only import
you can use is the built-in "convex:/_system/repl/wrappers.js" module in the
template.

The function call is also completely sandboxed, so it can only read data and
cannot modify the database or access the network.

Returns the result and any log lines generated by the query.
`.trim();

export const RunOneoffQueryTool: ConvexTool<
  typeof inputSchema,
  typeof outputSchema
> = {
  name: "runOneoffQuery",
  description,
  inputSchema,
  outputSchema,
  handler: async (ctx, args) => {
    const { projectDir, deployment } = await ctx.decodeDeploymentSelector(
      args.deploymentSelector,
    );
    process.chdir(projectDir);
    const deploymentSelection = await getDeploymentSelection(ctx, ctx.options);
    const credentials = await loadSelectedDeploymentCredentials(
      ctx,
      deploymentSelection,
      deployment,
    );
    try {
      const response = await fetch(`${credentials.url}/api/run_test_function`, {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
        },
        body: JSON.stringify({
          adminKey: credentials.adminKey,
          args: {},
          bundle: {
            path: "testQuery.js",
            source: args.query,
          },
          format: "convex_encoded_json",
        }),
      });
      if (!response.ok) {
        return await ctx.crash({
          exitCode: 1,
          errorType: "fatal",
          printedMessage: `HTTP error ${response.status}: ${await response.text()}`,
        });
      }
      const result = await response.json();
      if (result.status !== "success") {
        return await ctx.crash({
          exitCode: 1,
          errorType: "fatal",
          printedMessage: `Query failed: ${JSON.stringify(result)}`,
        });
      }
      return {
        result: result.value,
        logLines: result.logLines,
      };
    } catch (err) {
      return await ctx.crash({
        exitCode: 1,
        errorType: "fatal",
        printedMessage: `Failed to run query: ${(err as Error).toString().trim()}`,
      });
    }
  },
};
