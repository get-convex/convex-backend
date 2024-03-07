import { Command, Option } from "@commander-js/extra-typings";

declare module "@commander-js/extra-typings" {
  interface Command<
    Args extends any[] = [],
    // eslint-disable-next-line @typescript-eslint/ban-types
    Opts extends OptionValues = {},
  > {
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
