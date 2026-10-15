import { Command } from "@commander-js/extra-typings";
import { components } from "./commands/components.js";
import { packages } from "./commands/packages.js";
import { AnalyzerError } from "./lib/output.js";
import { analyzerVersion } from "./version.js";

const program = new Command("bundle-analyzer")
  .description(
    [
      "Analyze the code bundle for the Convex project in the current directory.",
    ].join("\n"),
  )
  .version(analyzerVersion)
  .addCommand(components)
  .addCommand(packages);

program.parseAsync(process.argv).catch((e: unknown) => {
  if (e instanceof AnalyzerError) {
    process.stderr.write(`✖ ${e.message}\n`);
    process.exit(1);
  }
  throw e;
});
