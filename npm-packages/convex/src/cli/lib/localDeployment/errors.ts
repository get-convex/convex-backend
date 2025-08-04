import { Context } from "../../../bundler/context.js";
import { logFailure, logMessage } from "../../../bundler/log.js";

export class LocalDeploymentError extends Error {}

export function printLocalDeploymentOnError(ctx: Context) {
  // Note: Not printing the error message here since it should already be printed by
  // ctx.crash.
  logFailure(ctx, `Hit an error while running local deployment.`);
  logMessage(
    ctx,
    "Your error has been reported to our team, and we'll be working on it.",
  );
  logMessage(
    ctx,
    "To opt out, run `npx convex disable-local-deployments`. Then re-run your original command.",
  );
}
