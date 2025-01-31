import { Command, Option, OptionValues } from "@commander-js/extra-typings";
import { OneoffCtx } from "../../bundler/context.js";

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
