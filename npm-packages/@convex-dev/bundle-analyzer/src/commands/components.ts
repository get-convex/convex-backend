import { Command } from "@commander-js/extra-typings";
import { logOutput, parsePositiveInteger, printJson } from "../lib/output.js";
import { AnalyzedBuild } from "../lib/model.js";
import {
  CODE_COLOR,
  MAP_COLOR,
  barCells,
  bold,
  colorFor,
  colored,
  dim,
  formatBytes,
  legend,
  plural,
  stackedBar,
  table,
  terminalWidth,
  wrapItems,
} from "../lib/format.js";
import { loadAnalysis } from "../lib/analysis.js";
import { jsonOption } from "../lib/command.js";

export const components = new Command("components")
  .summary("Size of each component's bundle for each runtime")
  .description(
    ["Size of each component's bundle for each runtime, largest first."].join(
      "\n",
    ),
  )
  .option("--top <n>", "Number of rows to show", parsePositiveInteger, 30)
  .addOption(jsonOption())
  .action(async (options) => {
    const { builds } = await loadAnalysis();
    if (options.json) {
      printJson(componentsJson(builds, options.top));
    } else {
      logOutput(componentsReport(builds, { top: options.top }));
    }
  });

export function componentsReport(
  builds: AnalyzedBuild[],
  options: { top: number },
): string {
  const width = terminalWidth();
  const all = componentSizes(builds);
  const rows = all.slice(0, options.top);
  const max = Math.max(1, ...rows.map((r) => r.bytes));
  const cells = barCells(width, 70, 24);
  const count = new Set(all.map((r) => r.component)).size;
  const total = all.reduce((n, r) => n + r.bytes, 0);
  const hidden = all.length - rows.length;
  return [
    wrapItems(
      [
        bold("Components"),
        dim(plural(count, "component")),
        dim(`${formatBytes(total)} of code and source maps`),
      ],
      dim(" · "),
      width,
    ),
    "",
    table(
      [
        { header: "Component", flexible: true },
        { header: "Runtime" },
        { header: "Modules", align: "right" },
        { header: "Size", align: "right" },
        { header: "Code", align: "right", optional: true },
        { header: "Source maps", align: "right", optional: true },
        {
          header: legend(
            [
              { label: "code", color: CODE_COLOR },
              { label: "source maps", color: MAP_COLOR },
            ],
            Number.POSITIVE_INFINITY,
          ),
          optional: true,
        },
      ],
      rows.map((r) => [
        colored(colorFor(r.component), r.component),
        r.kind,
        String(r.modules),
        formatBytes(r.bytes),
        dim(formatBytes(r.codeBytes)),
        dim(formatBytes(r.sourceMapBytes)),
        stackedBar(
          [
            { value: r.codeBytes, color: CODE_COLOR },
            { value: r.sourceMapBytes, color: MAP_COLOR },
          ],
          Math.max(1, Math.round((r.bytes / max) * cells)),
        ),
      ]),
      width,
    ),
    ...(hidden > 0 ? [dim(`… ${hidden} more (use --top)`)] : []),
  ].join("\n");
}

export function componentsJson(builds: AnalyzedBuild[], limit: number) {
  return componentSizes(builds).slice(0, limit);
}

export type ComponentSize = {
  /** Build id, `component:runtime`. */
  id: string;
  component: string;
  definitionPath: string;
  kind: AnalyzedBuild["kind"];
  modules: number;
  /** Code plus source map bytes. */
  bytes: number;
  codeBytes: number;
  sourceMapBytes: number;
  externalPackages: AnalyzedBuild["externalPackages"];
};

/** One row per component and runtime, largest first. */
export function componentSizes(builds: AnalyzedBuild[]): ComponentSize[] {
  return builds
    .map((b) => {
      const size: ComponentSize = {
        id: b.id,
        component: b.component.label,
        definitionPath: b.component.definitionPath,
        kind: b.kind,
        modules: b.outputs.length,
        bytes: 0,
        codeBytes: 0,
        sourceMapBytes: 0,
        externalPackages: b.externalPackages,
      };
      for (const o of b.outputs) {
        size.codeBytes += o.codeBytes;
        size.sourceMapBytes += o.sourceMapBytes;
      }
      size.bytes = size.codeBytes + size.sourceMapBytes;
      return size;
    })
    .sort((a, b) => b.bytes - a.bytes);
}
