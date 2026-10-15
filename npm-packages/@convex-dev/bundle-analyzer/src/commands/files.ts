import { Command } from "@commander-js/extra-typings";
import { logOutput, parsePositiveInteger, printJson } from "../lib/output.js";
import {
  AnalyzedBuild,
  AnalyzedInput,
  displayInputPath,
  displayPath,
  inputBytes,
  ownerLabel,
} from "../lib/model.js";
import {
  bar,
  barCells,
  bold,
  colored,
  dim,
  formatBytes,
  ownerColor,
  plural,
  table,
  terminalWidth,
  wrapItems,
} from "../lib/format.js";
import { loadAnalysis } from "../lib/analysis.js";
import { jsonOption } from "../lib/command.js";

export const files = new Command("files")
  .summary("Largest individual source files across your bundles")
  .description(
    [
      "Largest individual source files across your bundles.",
      "",
      "A file bundled into several modules or builds has one row for each, since each copy is uploaded. Files are ranked by code plus source maps.",
    ].join("\n"),
  )
  .option("--top <n>", "Number of files to show", parsePositiveInteger, 30)
  .addOption(jsonOption())
  .action(async (options) => {
    const { builds } = await loadAnalysis();
    if (options.json) {
      printJson(filesJson(builds, options.top));
    } else {
      logOutput(filesReport(builds, { top: options.top }));
    }
  });

export function filesReport(
  builds: AnalyzedBuild[],
  options: { top: number },
): string {
  const { top } = options;
  const width = terminalWidth();
  const all = fileUsage(builds).filter((f) => f.bytes > 0);
  const max = Math.max(1, ...all.map((f) => f.bytes));
  const cells = barCells(width, 90, 16);
  const rows = all.slice(0, top).map((f) => {
    const owner = ownerLabel(f);
    const color = ownerColor(f.origin, f.packageName ?? owner);
    return [
      displayInputPath(f),
      colored(color, owner),
      formatBytes(f.bytes),
      dim(formatBytes(f.bytesInOutput)),
      dim(formatBytes(f.sourceMapBytes)),
      bar(f.bytes / max, cells, color),
      dim(f.buildId),
      dim(f.output),
    ];
  });
  const hidden = all.length - rows.length;
  const total = all.reduce((n, f) => n + f.bytes, 0);
  return [
    wrapItems(
      [
        bold("Files"),
        dim(plural(all.length, "file")),
        dim(`${formatBytes(total)} of code and source maps`),
      ],
      dim(" · "),
      width,
    ),
    "",
    table(
      [
        { header: "File", flexible: true },
        { header: "Owner", flexible: true, truncate: "end" },
        { header: "Size", align: "right" },
        { header: "Code", align: "right", optional: true },
        { header: "Source maps", align: "right", optional: true },
        { header: "" },
        { header: "Build", flexible: true },
        { header: "Module", flexible: true },
      ],
      rows,
      width,
    ),
    ...(hidden > 0 ? [dim(`… ${hidden} more (use --top)`)] : []),
  ].join("\n");
}

export function filesJson(builds: AnalyzedBuild[], limit: number) {
  return fileUsage(builds)
    .filter((f) => f.bytes > 0)
    .slice(0, limit)
    .map((f) => ({
      path: f.path,
      display: displayPath(f.path, f),
      build: f.buildId,
      module: f.output,
      origin: f.origin,
      package: f.packageName,
      version: f.packageVersion,
      bytes: f.bytes,
      codeBytes: f.bytesInOutput,
      sourceMapBytes: f.sourceMapBytes,
    }));
}

export type FileUsage = AnalyzedInput & {
  buildId: string;
  output: string;
  /** Code plus source map bytes. */
  bytes: number;
};

/** Every input in every output, so a file bundled into two chunks appears twice. */
export function fileUsage(builds: AnalyzedBuild[]): FileUsage[] {
  const result: FileUsage[] = [];
  for (const b of builds) {
    for (const o of b.outputs) {
      for (const i of o.inputs) {
        result.push({
          ...i,
          buildId: b.id,
          output: o.path,
          bytes: inputBytes(i),
        });
      }
    }
  }
  return result.sort((a, b) => b.bytes - a.bytes);
}
