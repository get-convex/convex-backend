import { Command, Option, OptionValues } from "@commander-js/extra-typings";
import { OneoffCtx } from "../../bundler/context.js";
import {
  CONVEX_SELF_HOST_ADMIN_KEY_VAR_NAME,
  CONVEX_SELF_HOST_URL_VAR_NAME,
  parseInteger,
  parsePositiveInteger,
} from "./utils/utils.js";

declare module "@commander-js/extra-typings" {
  interface Command<Args extends any[] = [], Opts extends OptionValues = {}> {
    /**
     * For a command that talks to the configured dev deployment by default,
     * add flags for talking to prod, preview, or other deployment in the same
     * project.
     *
     * These flags are added to the end of `command` (ordering matters for `--help`
     * output). `action` should look like "Import data into" because it is prefixed
     * onto help strings.
     *
     * The options can be passed to `deploymentSelectionFromOptions`.
     *
     * NOTE: This method only exists at runtime if this file is imported.
     * To help avoid this bug, this method takes in an `ActionDescription` which
     * can only be constructed via `actionDescription` from this file.
     */
    addDeploymentSelectionOptions(action: ActionDescription): Command<
      Args,
      Opts & {
        url?: string;
        adminKey?: string;
        prod?: boolean;
        previewName?: string;
        deploymentName?: string;
      }
    >;

    /**
     * Adds options for the `dev` command that are not involved in picking a
     * deployment, so they can be used by `npx convex dev` and
     * `npx convex self-host dev`.
     */
    addDevOptions(): Command<
      Args,
      Opts & {
        tailLogs?: boolean;
        verbose?: boolean;
        run?: string;
        runComponent?: string;
        once: boolean;
        untilSuccess: boolean;
        traceEvents: boolean;
        typecheck: "enable" | "try" | "disable";
        typecheckComponents: boolean;
        debugBundlePath?: string;
        codegen: "enable" | "disable";
        liveComponentSources?: boolean;
      }
    >;

    /**
     * Adds options for the `deploy` command.
     */
    addDeployOptions(): Command<
      Args,
      Opts & {
        verbose?: boolean;
        dryRun?: boolean;
        yes?: boolean;
        typecheck: "enable" | "try" | "disable";
        typecheckComponents: boolean;
        codegen: "enable" | "disable";
        cmd?: string;
        cmdUrlEnvVarName?: string;
        debugBundlePath?: string;
        debug?: boolean;
        writePushRequest?: string;
        liveComponentSources?: boolean;
      }
    >;

    /**
     * Adds options for `self-host` subcommands.
     */
    addSelfHostOptions(): Command<
      Args,
      Opts & {
        url?: string;
        adminKey?: string;
        env?: string;
      }
    >;

    /**
     * Adds options and arguments for the `run` command.
     */
    addRunOptions(): Command<
      [...Args, string, string | undefined],
      Opts & {
        watch?: boolean;
        push?: boolean;
        identity?: string;
        typecheck: "enable" | "try" | "disable";
        typecheckComponents: boolean;
        codegen: "enable" | "disable";
        component?: string;
        liveComponentSources?: boolean;
      }
    >;

    /**
     * Adds options for the `import` command.
     */
    addImportOptions(): Command<
      [...Args, string],
      Opts & {
        table?: string;
        format?: "csv" | "jsonLines" | "jsonArray" | "zip";
        replace?: boolean;
        append?: boolean;
        replaceAll?: boolean;
        yes?: boolean;
        component?: string;
      }
    >;

    /**
     * Adds options for the `export` command.
     */
    addExportOptions(): Command<
      Args,
      Opts & {
        path: string;
        includeFileStorage?: boolean;
      }
    >;

    /**
     * Adds options for the `data` command.
     */
    addDataOptions(): Command<
      [...Args, string | undefined],
      Opts & {
        limit: number;
        order: "asc" | "desc";
        component?: string;
      }
    >;

    /**
     * Adds options for the `logs` command.
     */
    addLogsOptions(): Command<
      Args,
      Opts & {
        history: number;
        success: boolean;
      }
    >;

    /**
     * Adds options for the `network-test` command.
     */
    addNetworkTestOptions(): Command<
      Args,
      Opts & {
        timeout?: string;
        ipFamily?: string;
        speedTest?: boolean;
      }
    >;
  }
}

Command.prototype.addDeploymentSelectionOptions = function (
  action: ActionDescription,
) {
  return this.addOption(
    new Option("--url <url>")
      .conflicts(["--prod", "--preview-name", "--deployment-name"])
      .hideHelp(),
  )
    .addOption(new Option("--admin-key <adminKey>").hideHelp())
    .addOption(
      new Option(
        "--prod",
        action + " this project's production deployment.",
      ).conflicts(["--preview-name", "--deployment-name", "--url"]),
    )
    .addOption(
      new Option(
        "--preview-name <previewName>",
        action + " the preview deployment with the given name.",
      ).conflicts(["--prod", "--deployment-name", "--url"]),
    )
    .addOption(
      new Option(
        "--deployment-name <deploymentName>",
        action + " the specified deployment.",
      ).conflicts(["--prod", "--preview-name", "--url"]),
    ) as any;
};

declare const tag: unique symbol;
type ActionDescription = string & { readonly [tag]: "noop" };
export function actionDescription(action: string): ActionDescription {
  return action as any;
}

Command.prototype.addDevOptions = function () {
  return this.option("-v, --verbose", "Show full listing of changes")
    .addOption(
      new Option(
        "--typecheck <mode>",
        `Check TypeScript files with \`tsc --noEmit\`.`,
      )
        .choices(["enable", "try", "disable"] as const)
        .default("try" as const),
    )
    .option(
      "--typecheck-components",
      "Check TypeScript files within component implementations with `tsc --noEmit`.",
      false,
    )
    .addOption(
      new Option("--codegen <mode>", "Regenerate code in `convex/_generated/`")
        .choices(["enable", "disable"] as const)
        .default("enable" as const),
    )
    .option(
      "--once",
      "Execute only the first 3 steps, stop on any failure",
      false,
    )
    .option(
      "--until-success",
      "Execute only the first 3 steps, on failure watch for local and remote changes and retry steps 2 and 3",
      false,
    )
    .option(
      "--run <functionName>",
      "The identifier of the function to run in step 3, " +
        "like `init` or `dir/file:myFunction`",
    )
    .option(
      "--run-component <functionName>",
      "If --run is used and the function is in a component, the path the component tree defined in convex.config.ts. " +
        "Components are a beta feature. This flag is unstable and may change in subsequent releases.",
    )
    .addOption(
      new Option(
        "--tail-logs",
        "Tail this project's Convex logs in this terminal.",
      ),
    )
    .addOption(new Option("--trace-events").default(false).hideHelp())
    .addOption(new Option("--debug-bundle-path <path>").hideHelp())
    .addOption(new Option("--live-component-sources").hideHelp());
};

export async function normalizeDevOptions(
  ctx: OneoffCtx,
  cmdOptions: {
    verbose?: boolean;
    typecheck: "enable" | "try" | "disable";
    typecheckComponents?: boolean;
    codegen: "enable" | "disable";
    once?: boolean;
    untilSuccess: boolean;
    run?: string;
    runComponent?: string;
    tailLogs?: boolean;
    traceEvents: boolean;
    debugBundlePath?: string;
    liveComponentSources?: boolean;
  },
) {
  if (cmdOptions.runComponent && !cmdOptions.run) {
    return await ctx.crash({
      exitCode: 1,
      errorType: "fatal",
      printedMessage: "Can't specify `--run-component` option without `--run`",
    });
  }

  if (cmdOptions.debugBundlePath !== undefined && !cmdOptions.once) {
    return await ctx.crash({
      exitCode: 1,
      errorType: "fatal",
      printedMessage: "`--debug-bundle-path` can only be used with `--once`.",
    });
  }

  return {
    verbose: !!cmdOptions.verbose,
    typecheck: cmdOptions.typecheck,
    typecheckComponents: !!cmdOptions.typecheckComponents,
    codegen: cmdOptions.codegen === "enable",
    once: !!cmdOptions.once,
    untilSuccess: cmdOptions.untilSuccess,
    run: cmdOptions.run,
    runComponent: cmdOptions.runComponent,
    tailLogs: !!cmdOptions.tailLogs,
    traceEvents: cmdOptions.traceEvents,
    debugBundlePath: cmdOptions.debugBundlePath,
    liveComponentSources: !!cmdOptions.liveComponentSources,
  };
}

Command.prototype.addDeployOptions = function () {
  return this.option("-v, --verbose", "Show full listing of changes")
    .option(
      "--dry-run",
      "Print out the generated configuration without deploying to your Convex deployment",
    )
    .option("-y, --yes", "Skip confirmation prompt when running locally")
    .addOption(
      new Option(
        "--typecheck <mode>",
        `Whether to check TypeScript files with \`tsc --noEmit\` before deploying.`,
      )
        .choices(["enable", "try", "disable"] as const)
        .default("try" as const),
    )
    .option(
      "--typecheck-components",
      "Check TypeScript files within component implementations with `tsc --noEmit`.",
      false,
    )
    .addOption(
      new Option(
        "--codegen <mode>",
        "Whether to regenerate code in `convex/_generated/` before pushing.",
      )
        .choices(["enable", "disable"] as const)
        .default("enable" as const),
    )
    .addOption(
      new Option(
        "--cmd <command>",
        "Command to run as part of deploying your app (e.g. `vite build`). This command can depend on the environment variables specified in `--cmd-url-env-var-name` being set.",
      ),
    )
    .addOption(
      new Option(
        "--cmd-url-env-var-name <name>",
        "Environment variable name to set Convex deployment URL (e.g. `VITE_CONVEX_URL`) when using `--cmd`",
      ),
    )
    .addOption(new Option("--debug-bundle-path <path>").hideHelp())
    .addOption(new Option("--debug").hideHelp())
    .addOption(new Option("--write-push-request <writePushRequest>").hideHelp())
    .addOption(new Option("--live-component-sources").hideHelp());
};

Command.prototype.addSelfHostOptions = function () {
  return this.option(
    "--admin-key <adminKey>",
    `An admin key for the deployment. Can alternatively be set as \`${CONVEX_SELF_HOST_ADMIN_KEY_VAR_NAME}\` environment variable.`,
  )
    .option(
      "--url <url>",
      `The url of the deployment. Can alternatively be set as \`${CONVEX_SELF_HOST_URL_VAR_NAME}\` environment variable.`,
    )
    .option(
      "--env <env>",
      `Path to a custom file of environment variables, containing \`${CONVEX_SELF_HOST_URL_VAR_NAME}\` and \`${CONVEX_SELF_HOST_ADMIN_KEY_VAR_NAME}\`.`,
    );
};

Command.prototype.addRunOptions = function () {
  return (
    this.argument(
      "functionName",
      "identifier of the function to run, like `listMessages` or `dir/file:myFunction`",
    )
      .argument(
        "[args]",
        "JSON-formatted arguments object to pass to the function.",
      )
      .option(
        "-w, --watch",
        "Watch a query, printing its result if the underlying data changes. Given function must be a query.",
      )
      .option("--push", "Push code to deployment before running the function.")
      .addOption(
        new Option(
          "--identity <identity>",
          'JSON-formatted UserIdentity object, e.g. \'{ name: "John", address: "0x123" }\'',
        ),
      )
      // For backwards compatibility we still support --no-push which is a noop
      .addOption(new Option("--no-push").hideHelp())
      // Options for the deploy that --push does
      .addOption(
        new Option(
          "--typecheck <mode>",
          `Whether to check TypeScript files with \`tsc --noEmit\`.`,
        )
          .choices(["enable", "try", "disable"] as const)
          .default("try" as const),
      )
      .option(
        "--typecheck-components",
        "Check TypeScript files within component implementations with `tsc --noEmit`.",
        false,
      )
      .addOption(
        new Option(
          "--codegen <mode>",
          "Regenerate code in `convex/_generated/`",
        )
          .choices(["enable", "disable"] as const)
          .default("enable" as const),
      )
      .addOption(
        new Option(
          "--component <path>",
          "Path to the component in the component tree defined in convex.config.ts. " +
            "Components are a beta feature. This flag is unstable and may change in subsequent releases.",
        ),
      )
      .addOption(new Option("--live-component-sources").hideHelp())
  );
};

Command.prototype.addImportOptions = function () {
  return this.argument("<path>", "Path to the input file")
    .addOption(
      new Option(
        "--table <table>",
        "Destination table name. Required if format is csv, jsonLines, or jsonArray. Not supported if format is zip.",
      ),
    )
    .addOption(
      new Option(
        "--replace",
        "Replace all existing data in any of the imported tables",
      )
        .conflicts("--append")
        .conflicts("--replace-all"),
    )
    .addOption(
      new Option("--append", "Append imported data to any existing tables")
        .conflicts("--replace-all")
        .conflicts("--replace"),
    )
    .addOption(
      new Option(
        "--replace-all",
        "Replace all existing data in the deployment with the imported tables,\n" +
          "  deleting tables that don't appear in the import file or the schema,\n" +
          "  and clearing tables that appear in the schema but not in the import file",
      )
        .conflicts("--append")
        .conflicts("--replace"),
    )
    .option(
      "-y, --yes",
      "Skip confirmation prompt when import leads to deleting existing documents",
    )
    .addOption(
      new Option(
        "--format <format>",
        "Input file format. This flag is only required if the filename is missing an extension.\n" +
          "- CSV files must have a header, and each row's entries are interpreted either as a (floating point) number or a string.\n" +
          "- JSON files must be an array of JSON objects.\n" +
          "- JSONLines files must have a JSON object per line.\n" +
          "- ZIP files must have one directory per table, containing <table>/documents.jsonl. Snapshot exports from the Convex dashboard have this format.",
      ).choices(["csv", "jsonLines", "jsonArray", "zip"]),
    )
    .addOption(
      new Option(
        "--component <path>",
        "Path to the component in the component tree defined in convex.config.ts",
      ),
    );
};

Command.prototype.addExportOptions = function () {
  return this.requiredOption(
    "--path <zipFilePath>",
    "Exports data into a ZIP file at this path, which may be a directory or unoccupied .zip path",
  ).addOption(
    new Option(
      "--include-file-storage",
      "Includes stored files (https://dashboard.convex.dev/deployment/files) in a _storage folder within the ZIP file",
    ),
  );
};

Command.prototype.addDataOptions = function () {
  return this.addOption(
    new Option(
      "--limit <n>",
      "List only the `n` the most recently created documents.",
    )
      .default(100)
      .argParser(parsePositiveInteger),
  )
    .addOption(
      new Option(
        "--order <choice>",
        "Order the documents by their `_creationTime`.",
      )
        .choices(["asc", "desc"])
        .default("desc"),
    )
    .addOption(
      new Option(
        "--component <path>",
        "Path to the component in the component tree defined in convex.config.ts.\n" +
          "  By default, inspects data in the root component",
      ).hideHelp(),
    )
    .argument("[table]", "If specified, list documents in this table.");
};

Command.prototype.addLogsOptions = function () {
  return this.option(
    "--history [n]",
    "Show `n` most recent logs. Defaults to showing all available logs.",
    parseInteger,
  ).option(
    "--success",
    "Print a log line for every successful function execution",
    false,
  );
};

Command.prototype.addNetworkTestOptions = function () {
  return this.addOption(
    new Option(
      "--timeout <timeout>",
      "Timeout in seconds for the network test (default: 30).",
    ),
  )
    .addOption(
      new Option(
        "--ip-family <ipFamily>",
        "IP family to use (ipv4, ipv6, or auto)",
      ),
    )
    .addOption(
      new Option(
        "--speed-test",
        "Perform a large echo test to measure network speed.",
      ),
    );
};
