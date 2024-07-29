import { Command, Option } from "@commander-js/extra-typings";
import { logFailure, oneoffContext } from "../bundler/context.js";

// Reinitialize an existing Convex project.
// This command is deprecated and hidden from the command help.
// `npx convex dev --once --configure=existing` replaces it.
export const reinit = new Command("reinit")
  .description(
    "Reinitialize a Convex project in the local directory if you've lost your convex.json file",
  )
  .addOption(
    new Option(
      "--team <team_slug>",
      "The identifier of the team the project belongs to.",
    ),
  )
  .addOption(
    new Option(
      "--project <project_slug>",
      "The identifier of the project you'd like to reinitialize.",
    ),
  )
  .action(async (_options) => {
    logFailure(
      oneoffContext,
      "The `reinit` command is deprecated. Use `npx convex dev --once --configure=existing` instead.",
    );
    return oneoffContext.crash(
      1,
      "fatal",
      "The `reinit` command is deprecated. Use `npx convex dev --once --configure=existing` instead.",
    );
  });
