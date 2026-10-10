import { Command } from "@commander-js/extra-typings";
import { AnalyzerError } from "./output.js";
import { analyzerVersion } from "./version.js";

const program = new Command("bundle-analyzer")
  .description(
    [
      "Analyze the code bundle for the Convex project in the current directory.",
    ].join("\n"),
  )
  .version(analyzerVersion);

program.parseAsync(process.argv).catch((e: unknown) => {
  if (e instanceof AnalyzerError) {
    process.stderr.write(`✖ ${e.message}\n`);
    process.exit(1);
  }
  throw e;
});
