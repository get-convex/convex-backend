import { Command } from "@commander-js/extra-typings";
import { logOutput, parsePositiveInteger, printJson } from "../lib/output.js";
import {
  AnalyzedBuild,
  InputAttribution,
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
      "A file bundled into several modules or builds counts once for each copy, since each copy is uploaded. Files are ranked by code plus source maps.",
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
  const cells = barCells(width, 50, 16);
  const rows = all.slice(0, top).map((f) => {
    const color = ownerColor(f.origin, f.packageName ?? ownerLabel(f));
    return [
      colored(color, f.display),
      formatBytes(f.bytes),
      dim(formatBytes(f.codeBytes)),
      dim(formatBytes(f.sourceMapBytes)),
      bar(f.bytes / max, cells, color),
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
        { header: "Size", align: "right" },
        { header: "Code", align: "right", optional: true },
        { header: "Source maps", align: "right", optional: true },
        { header: "", optional: true },
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
      display: f.display,
      origin: f.origin,
      package: f.packageName,
      version: f.packageVersion,
      bytes: f.bytes,
      codeBytes: f.codeBytes,
      sourceMapBytes: f.sourceMapBytes,
      copies: f.copies,
    }));
}

/** One copy of a file: its code in one module of one build. */
export type FileCopy = {
  buildId: string;
  module: string;
  /** Code plus source map bytes. */
  bytes: number;
  codeBytes: number;
  sourceMapBytes: number;
};

export type FileUsage = InputAttribution & {
  /** Metafile input path. */
  path: string;
  /** The path with its package, e.g. `zod@3.25.76/index.js`. */
  display: string;
  /** Code plus source map bytes, summed over copies. */
  bytes: number;
  codeBytes: number;
  sourceMapBytes: number;
  /** Largest first. */
  copies: FileCopy[];
};

/**
 * Each input file, summed over every module and build it's bundled into. A
 * file attributed differently in different builds (a component's own code is
 * `your code` in its builds but the component's package in the app's) gets a
 * row per attribution, the same split `packages` makes.
 */
export function fileUsage(builds: AnalyzedBuild[]): FileUsage[] {
  const byKey = new Map<string, FileUsage>();
  for (const b of builds) {
    for (const o of b.outputs) {
      for (const i of o.inputs) {
        const key = `${i.path}\0${ownerLabel(i)}`;
        let usage = byKey.get(key);
        if (!usage) {
          usage = {
            origin: i.origin,
            packageName: i.packageName,
            packageVersion: i.packageVersion,
            packageDir: i.packageDir,
            path: i.path,
            display: displayPath(i.path, i),
            bytes: 0,
            codeBytes: 0,
            sourceMapBytes: 0,
            copies: [],
          };
          byKey.set(key, usage);
        }
        const bytes = inputBytes(i);
        usage.bytes += bytes;
        usage.codeBytes += i.bytesInOutput;
        usage.sourceMapBytes += i.sourceMapBytes;
        usage.copies.push({
          buildId: b.id,
          module: o.path,
          bytes,
          codeBytes: i.bytesInOutput,
          sourceMapBytes: i.sourceMapBytes,
        });
      }
    }
  }
  const result = [...byKey.values()];
  for (const f of result) f.copies.sort((a, b) => b.bytes - a.bytes);
  return result.sort(
    (a, b) =>
      b.bytes - a.bytes ||
      a.display.localeCompare(b.display) ||
      a.path.localeCompare(b.path),
  );
}
