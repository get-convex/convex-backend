import { Command } from "@commander-js/extra-typings";
import {
  logError,
  logFinishedStep,
  logMessage,
  oneoffContext,
} from "../bundler/context.js";
import { Server } from "@modelcontextprotocol/sdk/server/index.js";
import { StdioServerTransport } from "@modelcontextprotocol/sdk/server/stdio.js";
import { actionDescription } from "./lib/command.js";
import { DeploymentSelectionOptions } from "./lib/api.js";
import { checkAuthorization } from "./lib/login.js";
import {
  CallToolRequest,
  CallToolRequestSchema,
  ListToolsRequestSchema,
} from "@modelcontextprotocol/sdk/types.js";
import { RequestContext, RequestCrash } from "./lib/mcp/requestContext.js";
import { mcpTool, convexTools } from "./lib/mcp/tools/index.js";
import * as path from "path";
import chalk from "chalk";

export const mcp = new Command("mcp")
  .summary("Manage the Model Context Protocol server for Convex [BETA]")
  .description(
    "Commands to initialize and run a Model Context Protocol server for Convex that can be used with AI tools.\n" +
      "This server exposes your Convex codebase to AI tools in a structured way.",
  )
  .allowExcessArguments(false);

// Init command to set up Cursor MCP configuration.
mcp
  .command("init-cursor")
  .summary("Initialize the MCP server configuration for Cursor")
  .description("Set up the .cursor/mcp.json file for the current project.")
  .addDeploymentSelectionOptions(actionDescription("Run the function on"))
  .action(async (cmdOptions) => {
    const ctx = oneoffContext();

    const scriptRelativePath = "node_modules/convex/bin/main.js";
    if (!ctx.fs.exists(scriptRelativePath)) {
      logError(ctx, "Failed to find Convex script path.");
      return;
    }
    const scriptPath = path.resolve(scriptRelativePath);

    // Cursor is much more finicky than `@modelcontextprotocol/inspector` about
    // setting up the server correctly:
    // 1. It runs everything under working directory `/`, so we need to pass in
    //    the project's directory manually.
    // 2. It doesn't seem to have `npx` in path, so we call `node` directly and
    //    pass in the path to the CLI's main file.
    const projectDir = process.cwd();

    const args = [scriptPath, "mcp", "start", projectDir];
    if (cmdOptions.prod) {
      args.push("--prod");
    }
    if (cmdOptions.adminKey) {
      args.push("--admin-key", cmdOptions.adminKey);
    }
    if (cmdOptions.url) {
      args.push("--url", cmdOptions.url);
    }
    if (cmdOptions.deploymentName) {
      args.push("--deployment-name", cmdOptions.deploymentName);
    }
    if (cmdOptions.previewName) {
      args.push("--preview-name", cmdOptions.previewName);
    }
    if (cmdOptions.deploymentName) {
      args.push("--deployment-name", cmdOptions.deploymentName);
    }
    const convexServer = {
      command: "node",
      args,
    };
    const mcpPath = path.join(".cursor", "mcp.json");
    ctx.fs.mkdir(path.dirname(mcpPath), {
      allowExisting: true,
      recursive: true,
    });
    if (ctx.fs.exists(mcpPath)) {
      try {
        const existing = ctx.fs.readUtf8File(mcpPath);
        const existingConfig = JSON.parse(existing);
        if (existingConfig.mcpServers?.convex) {
          return await ctx.crash({
            exitCode: 1,
            errorType: "fatal",
            printedMessage:
              "Convex MCP configuration already exists in '.cursor/mcp.json'.",
          });
        }
        if (!existingConfig.mcpServers) {
          existingConfig.mcpServers = {};
        }
        existingConfig.mcpServers.convex = convexServer;
        ctx.fs.writeUtf8File(mcpPath, JSON.stringify(existingConfig, null, 2));
        logFinishedStep(
          ctx,
          "Added `convex` to existing MCP configuration at '.cursor/mcp.json'.",
        );
      } catch (error: any) {
        return await ctx.crash({
          exitCode: 1,
          errorType: "fatal",
          printedMessage: `Failed to parse existing MCP configuration: ${error.toString()}`,
        });
      }
    } else {
      const config = { mcpServers: { convex: convexServer } };
      ctx.fs.writeUtf8File(mcpPath, JSON.stringify(config, null, 2));
      logFinishedStep(
        ctx,
        "Created new MCP configuration at '.cursor/mcp.json'.",
      );
    }
    logMessage(
      ctx,
      chalk.bold(
        "Next, open Cursor Settings > MCP, and enable the Convex MCP server.",
      ),
    );
  });

// Start command
mcp
  .command("start")
  .summary("Start the MCP server")
  .description(
    "Start the Model Context Protocol server for Convex that can be used with AI tools.",
  )
  .argument("<project-dir>", "Path to the project directory")
  .addDeploymentSelectionOptions(actionDescription("Run the function on"))
  .action(async (projectDir, cmdOptions) => {
    const ctx = oneoffContext();
    try {
      process.chdir(projectDir);
      const server = makeServer(cmdOptions);
      const transport = new StdioServerTransport();
      await server.connect(transport);
      // Keep the process running
      await new Promise(() => {});
    } catch (error: any) {
      await ctx.crash({
        exitCode: 1,
        errorType: "fatal",
        errForSentry: `Failed to start MCP server: ${error}`,
        printedMessage: `Failed to start MCP server: ${error}`,
      });
    }
  });

function makeServer(cmdOptions: DeploymentSelectionOptions) {
  const convexToolsByName = Object.fromEntries(
    convexTools.map((tool) => [tool.name, tool]),
  );
  const server = new Server(
    {
      name: "Convex MCP Server",
      version: "0.0.1",
    },
    {
      capabilities: {
        tools: {},
      },
    },
  );
  server.setRequestHandler(
    CallToolRequestSchema,
    async (request: CallToolRequest) => {
      const ctx = new RequestContext(cmdOptions);
      try {
        const authorized = await checkAuthorization(ctx, false);
        if (!authorized) {
          await ctx.crash({
            exitCode: 1,
            errorType: "fatal",
            printedMessage:
              "Not Authorized: Run `npx convex dev` to login to your Convex project.",
          });
        }
        if (!request.params.arguments) {
          await ctx.crash({
            exitCode: 1,
            errorType: "fatal",
            printedMessage: "No arguments provided",
          });
        }
        const convexTool = convexToolsByName[request.params.name];
        if (!convexTool) {
          await ctx.crash({
            exitCode: 1,
            errorType: "fatal",
            printedMessage: `Tool ${request.params.name} not found`,
          });
        }
        const input = convexTool.inputSchema.parse(request.params.arguments);
        const result = await convexTool.handler(ctx, input);
        return {
          content: [
            {
              type: "text",
              text: JSON.stringify(result),
            },
          ],
        };
      } catch (error: any) {
        let message: string;
        if (error instanceof RequestCrash) {
          message = error.printedMessage;
        } else if (error instanceof Error) {
          message = error.message;
        } else {
          message = String(error);
        }
        return {
          content: [
            {
              type: "text",
              text: JSON.stringify({ error: message }),
            },
          ],
          isError: true,
        };
      }
    },
  );
  server.setRequestHandler(ListToolsRequestSchema, async () => {
    return {
      tools: convexTools.map(mcpTool),
    };
  });
  return server;
}
