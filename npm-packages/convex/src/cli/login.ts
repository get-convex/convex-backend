import { Command, Option } from "@commander-js/extra-typings";
import { logFinishedStep, oneoffContext } from "../bundler/context.js";
import { checkAuthorization, performLogin } from "./lib/login.js";

export const login = new Command("login")
  .description("Login to Convex")
  .option(
    "--device-name <name>",
    "Provide a name for the device being authorized",
  )
  .option(
    "-f, --force",
    "Proceed with login even if a valid access token already exists for this device",
  )
  .option(
    "--no-open",
    "Don't automatically open the login link in the default browser",
  )
  // These options are hidden from the help/usage message, but allow overriding settings for testing.
  // Change the auth credentials with the auth provider
  .addOption(new Option("--override-auth-url <url>").hideHelp())
  .addOption(new Option("--override-auth-client <id>").hideHelp())
  .addOption(new Option("--override-auth-username <username>").hideHelp())
  .addOption(new Option("--override-auth-password <password>").hideHelp())
  // Skip the auth provider login and directly use this access token
  .addOption(new Option("--override-access-token <token>").hideHelp())
  // Automatically accept opt ins without prompting
  .addOption(new Option("--accept-opt-ins").hideHelp())
  // Dump the access token from the auth provider and skip authorization with Convex
  .addOption(new Option("--dump-access-token").hideHelp())
  .action(async (options, cmd: Command) => {
    const ctx = oneoffContext;
    if (
      !options.force &&
      (await checkAuthorization(ctx, !!options.acceptOptIns))
    ) {
      logFinishedStep(
        ctx,
        "This device has previously been authorized and is ready for use with Convex.",
      );
      return;
    }
    if (!!options.overrideAuthUsername !== !!options.overrideAuthPassword) {
      cmd.error(
        "If overriding credentials, both username and password must be provided",
      );
    }

    await performLogin(ctx, options);
  });
