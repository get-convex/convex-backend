import { Command } from "@commander-js/extra-typings";
import { logOutput, parsePositiveInteger, printJson } from "../lib/output.js";
import { AnalyzedBuild } from "../lib/model.js";
import {
  bold,
  dim,
  formatBytes,
  plural,
  prose,
  table,
  terminalWidth,
  wrapItems,
} from "../lib/format.js";
import { loadAnalysis } from "../lib/analysis.js";
import { jsonOption } from "../lib/command.js";
import {
  ChainTarget,
  chainTargets,
  chainTargetsJson,
  importedBySection,
  matchesPackage,
} from "../lib/importChains.js";
import { packagesTable, packagesJson, packageUsage } from "../lib/packages.js";
import { FileUsage, fileUsage } from "./files.js";

export const packages = new Command("packages")
  .summary("Largest npm packages (and your own code) across your bundles")
  .description(
    [
      "Largest npm packages (and your own code) across your bundles.",
      "",
      "Each version of a package is a separate row. Packages are ranked by code plus source maps, since both are uploaded and count toward the deployment size limits.",
      "",
      "Pass a package to see its largest files and the entry points that import each copy of it, each with its shortest import chain. `--limit` sets how many entry points to show. Copies installed in different directories are listed separately, even at the same version.",
    ].join("\n"),
  )
  .argument(
    "[package]",
    "Package name, optionally with a version (e.g. `zod` or `zod@3.25.76`)",
  )
  .option(
    "--top <n>",
    "Number of packages (or of a package's files) to show",
    parsePositiveInteger,
    30,
  )
  .option(
    "--limit <n>",
    "Entry points to show per package copy, each with its import chain",
    parsePositiveInteger,
    3,
  )
  .addOption(jsonOption())
  .action(async (name, options) => {
    const { builds } = await loadAnalysis();
    const { top, limit } = options;
    if (name !== undefined) {
      const detail = packageDetail(builds, name);
      if (options.json) {
        printJson(packageDetailJson(detail, { top, limit }));
      } else {
        logOutput(packageDetailReport(detail, { top, limit }));
      }
    } else if (options.json) {
      printJson(packagesJson(builds, top));
    } else {
      logOutput(packagesReport(builds, { top }));
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
    "",
    dim(
      prose(
        "Run `npx @convex-dev/bundle-analyzer packages <package>` to see a package's files and what imports it.",
        width,
      ),
    ),
  ].join("\n");
}

export type PackageDetail = {
  name: string;
  /** Each copy of the package in each build, with its import chains. */
  copies: ChainTarget[];
  /** The package's nonempty files, summed over every build, largest first. */
  files: FileUsage[];
  /** External node packages with this name, which aren't bundled. */
  external: { buildId: string; name: string; version: string }[];
};

export function packageDetail(
  builds: AnalyzedBuild[],
  name: string,
): PackageDetail {
  return {
    name,
    copies: chainTargets(builds, { kind: "package", name }),
    files: fileUsage(builds).filter(
      (f) => f.bytes > 0 && matchesPackage(f, name),
    ),
    external: builds.flatMap((b) =>
      b.externalPackages
        .filter((e) => e.name === name || `${e.name}@${e.version}` === name)
        .map((e) => ({ buildId: b.id, name: e.name, version: e.version })),
    ),
  };
}

export function packageDetailJson(
  detail: PackageDetail,
  options: { top: number; limit: number },
) {
  return {
    name: detail.name,
    copies: chainTargetsJson(detail.copies, options.limit),
    files: detail.files.slice(0, options.top).map((f) => ({
      path: f.path,
      display: f.display,
      bytes: f.bytes,
      codeBytes: f.codeBytes,
      sourceMapBytes: f.sourceMapBytes,
    })),
    external: detail.external.map((e) => ({
      build: e.buildId,
      name: e.name,
      version: e.version,
    })),
  };
}

export function packageDetailReport(
  detail: PackageDetail,
  options: { top: number; limit: number },
): string {
  const { top, limit } = options;
  const width = terminalWidth();
  const name = bold(detail.name);
  const externalIn = [...new Set(detail.external.map((e) => e.buildId))];
  if (detail.copies.length === 0) {
    if (externalIn.length > 0) {
      return prose(
        `${name} isn't bundled: it's an external package in ${externalIn.join(", ")}, installed on the server instead of uploaded.`,
        width,
      );
    }
    return [
      prose(`No bundled package is named ${name}.`, width),
      dim(
        prose(
          "Use a package name (e.g. `zod`) or name and version (e.g. `zod@3.25.76`). To search file paths, use `npx @convex-dev/bundle-analyzer files <path>`.",
          width,
        ),
      ),
    ].join("\n");
  }

  const copies = new Set(
    detail.copies.map((c) => `${c.label}\0${c.packageDir}`),
  ).size;
  const builds = new Set(detail.copies.map((c) => c.buildId)).size;
  const total = detail.copies.reduce((n, c) => n + c.bytes, 0);
  const rows = detail.files.slice(0, top);
  const hidden = detail.files.length - rows.length;
  const out = [
    wrapItems(
      [
        name,
        dim(copies === 1 ? "1 copy" : `${copies} copies`),
        dim(plural(builds, "build")),
        dim(`${formatBytes(total)} of code and source maps`),
      ],
      dim(" · "),
      width,
    ),
    "",
    bold("Largest files"),
    "",
    table(
      [
        { header: "File", flexible: true },
        { header: "Size", align: "right" },
        { header: "Code", align: "right", optional: true },
        { header: "Source maps", align: "right", optional: true },
      ],
      rows.map((f) => [
        f.display,
        formatBytes(f.bytes),
        dim(formatBytes(f.codeBytes)),
        dim(formatBytes(f.sourceMapBytes)),
      ]),
      width,
    ),
    ...(hidden > 0 ? [dim(`… ${hidden} more (use --top)`)] : []),
    "",
    ...importedBySection(detail.copies, { limit, width, showTargets: true }),
  ];
  if (externalIn.length > 0) {
    out.push(
      "",
      dim(
        prose(
          `It's also an external package in ${externalIn.join(", ")}, installed on the server instead of uploaded.`,
          width,
        ),
      ),
    );
  }
  out.push(
    "",
    dim(
      prose(
        "Run `npx @convex-dev/bundle-analyzer files <path>` to see where a file ends up.",
        width,
      ),
    ),
  );
  return out.join("\n");
}
