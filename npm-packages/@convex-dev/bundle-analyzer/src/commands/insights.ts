import fs from "node:fs";
import path from "node:path";
import { Command } from "@commander-js/extra-typings";
import { logOutput, printJson } from "../lib/output.js";
import { loadAnalysis } from "../lib/analysis.js";
import { jsonOption } from "../lib/command.js";
import { insightsReport, findInsights } from "../lib/insights.js";

export const insights = new Command("insights")
  .summary("Suggestions for making your bundles smaller")
  .description(
    [
      'Suggestions for making your bundles smaller: duplicated packages, "use node" files bundled into the Convex runtime, heavy modules, devDependencies and test files that functions import, and large packages that could be external or imported lazily.',
      "",
      "Package and duplicate sizes count code plus source maps, since both are uploaded. Module load sizes count code, since that's what's evaluated.",
    ].join("\n"),
  )
  .addOption(jsonOption())
  .action(async (options) => {
    const { analysis, builds } = await loadAnalysis();
    const found = findInsights(
      builds,
      devDependencies(analysis.bundle.projectRoot),
    );
    if (options.json) {
      printJson({ insights: found });
    } else {
      logOutput(
        found.length > 0 ? insightsReport(found) : "Nothing stands out.",
      );
    }
  });

function devDependencies(projectRoot: string): Set<string> {
  try {
    const pkg = JSON.parse(
      fs.readFileSync(path.join(projectRoot, "package.json"), "utf8"),
    );
    return new Set(Object.keys(pkg.devDependencies ?? {}));
  } catch {
    return new Set();
  }
}
