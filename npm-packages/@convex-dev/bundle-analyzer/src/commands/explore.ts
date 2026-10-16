import { Command } from "@commander-js/extra-typings";
import { runExplorer } from "../lib/tui.js";
import { AnalyzerError } from "../lib/output.js";
import { loadAnalysis } from "../lib/analysis.js";
import { jsonOption } from "../lib/command.js";

export const explore = new Command("explore")
  .summary("Browse your bundles interactively in the terminal")
  .description(
    [
      "Browse your bundles in an interactive terminal UI: drill down from components and runtimes to modules, packages and files, with a live treemap of the current level and the import chain that pulls in each file.",
      "",
      "Sizes count code plus source maps. Press t to switch layouts, g to group by component or by package, / to filter, and ? for all keys.",
    ].join("\n"),
  )
  // Accepted only so it can be refused with a pointer to the commands
  // that have JSON output.
  .addOption(jsonOption().hideHelp())
  .action(async (options) => {
    if (options.json || !process.stdin.isTTY || !process.stdout.isTTY) {
      throw new AnalyzerError(
        options.json
          ? "`explore` is interactive and has no JSON output. Every other command supports --json."
          : "`explore` needs an interactive terminal. Every other command prints a report instead, and supports --json.",
      );
    }
    const { builds } = await loadAnalysis();
    await runExplorer(builds);
  });
