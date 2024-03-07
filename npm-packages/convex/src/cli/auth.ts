import { Command, Option } from "@commander-js/extra-typings";
import { logFailure, oneoffContext } from "../bundler/context.js";

const list = new Command("list").action(async () => {
  const ctx = oneoffContext;
  logFailure(
    ctx,
    "convex auth commands were removed, see https://docs.convex.dev/auth for up to date instructions.",
  );
  await ctx.crash(1, "fatal", "Ran deprecated `convex auth list`");
});

const rm = new Command("remove").action(async () => {
  const ctx = oneoffContext;
  logFailure(
    ctx,
    "convex auth commands were removed, see https://docs.convex.dev/auth for up to date instructions.",
  );
  await ctx.crash(1, "fatal", "Ran deprecated `convex auth remove`");
});

const add = new Command("add")
  .addOption(new Option("--identity-provider-url <url>").hideHelp())
  .addOption(new Option("--application-id <applicationId>").hideHelp())
  .action(async () => {
    const ctx = oneoffContext;
    logFailure(
      ctx,
      "convex auth commands were removed, see https://docs.convex.dev/auth for up to date instructions.",
    );
    await ctx.crash(1, "fatal", "Ran deprecated `convex auth add`");
  });

export const auth = new Command("auth")
  .addCommand(list)
  .addCommand(rm)
  .addCommand(add);
