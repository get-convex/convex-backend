import { Command } from "@commander-js/extra-typings";
import { logOutput, parsePositiveInteger, printJson } from "../lib/output.js";
import { AnalyzedBuild } from "../lib/model.js";
import {
  bold,
  dim,
  formatBytes,
  plural,
  terminalWidth,
  wrapItems,
} from "../lib/format.js";
import { loadAnalysis } from "../lib/analysis.js";
import { jsonOption } from "../lib/command.js";
import { packagesTable, packagesJson, packageUsage } from "../lib/packages.js";

export const packages = new Command("packages")
  .summary("Largest npm packages (and your own code) across your bundles")
  .description(
    [
      "Largest npm packages (and your own code) across your bundles.",
      "",
      "Each version of a package is a separate row. Packages are ranked by code plus source maps, since both are uploaded and count toward the deployment size limits.",
    ].join("\n"),
  )
  .option("--top <n>", "Number of packages to show", parsePositiveInteger, 30)
  .addOption(jsonOption())
  .action(async (options) => {
    const { builds } = await loadAnalysis();
    if (options.json) {
      printJson(packagesJson(builds, options.top));
    } else {
      logOutput(packagesReport(builds, { top: options.top }));
    }
  });

export function packagesReport(
  builds: AnalyzedBuild[],
  options: { top: number },
): string {
  const { top } = options;
  const all = packageUsage(builds).filter((p) => p.bytes > 0);
  const total = all.reduce((n, p) => n + p.bytes, 0);
  const width = terminalWidth();
  return [
    wrapItems(
      [
        bold("Packages"),
        dim(plural(all.length, "package")),
        dim(`${formatBytes(total)} of code and source maps`),
      ],
      dim(" · "),
      width,
    ),
    "",
    packagesTable(builds, top, width),
  ].join("\n");
}
