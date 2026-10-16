import { Command, Option } from "@commander-js/extra-typings";
import { logOutput, parsePositiveInteger, printJson } from "../lib/output.js";
import { AnalyzedBuild } from "../lib/model.js";
import {
  CODE_COLOR,
  barCells,
  bold,
  colored,
  dim,
  formatBytes,
  formatPercent,
  heading,
  legend,
  ownerColor,
  plural,
  prose,
  stackedBar,
  table,
  terminalWidth,
  waterfallBar,
  wrapItems,
} from "../lib/format.js";
import { loadAnalysis } from "../lib/analysis.js";
import { jsonOption } from "../lib/command.js";
import {
  FUNCTION_RUNTIMES,
  FunctionRuntime,
  ImportedModule,
  ModuleDetail,
  ModuleLoader,
  SHARED_CHUNK_COLOR,
  lookupModules,
  moduleLoads,
  modulesTable,
} from "../lib/modules.js";

export const modules = new Command("modules")
  .summary(
    "Load size of each module: the module plus the shared chunks it imports",
  )
  .description(
    [
      "Load size of each function module: what the runtime loads to run a function in it.",
      "",
      "Own is the module's own file. Imports is every shared chunk it imports statically, directly or indirectly. Modules are ranked by the two together, which is what the runtime loads: the Convex runtime compiles and evaluates exactly this code on every call. Node evaluates each module once per warm instance, and loads packages left external from node_modules on top of it.",
      "",
      "Pass a module, either a function module (e.g. `convex/messages.ts`) or a chunk esbuild split out (e.g. `_deps/3565P6XD.js`), to see the files in it, the modules it imports and what dynamic `import()`s would add, and which function modules load it. Actions can use `import()`; queries and mutations can't.",
      "",
      "Sizes count code only. Source maps are stored and downloaded with the code but aren't compiled or evaluated.",
    ].join("\n"),
  )
  .argument(
    "[module]",
    "Module or chunk path, or part of one (e.g. `convex/messages.ts`, `messages.js`, or `_deps/3565P6XD.js`)",
  )
  .addOption(
    new Option(
      "--runtime <runtime>",
      "Only show one runtime's modules",
    ).choices(FUNCTION_RUNTIMES),
  )
  .option(
    "--top <n>",
    "Number of modules to show, or of a module's imports or a chunk's files",
    parsePositiveInteger,
    30,
  )
  .addOption(jsonOption())
  .action(async (module, options) => {
    const { builds } = await loadAnalysis();
    const { runtime, top } = options;
    if (module !== undefined) {
      const found = lookupModules(builds, module).filter(
        (d) => runtime === undefined || d.kind === runtime,
      );
      if (options.json) {
        printJson(lookupJson(found, top));
      } else {
        logOutput(lookupReport(found, module, top));
      }
    } else if (options.json) {
      printJson(modulesJson(builds, top, runtime));
    } else {
      logOutput(modulesReport(builds, { top, runtime }));
    }
  });

const LAZY_NOTES: Record<FunctionRuntime, string> = {
  convex: "when an action calls import()",
  node: "when import() runs",
};

export function modulesReport(
  builds: AnalyzedBuild[],
  options: { top: number; runtime?: FunctionRuntime | undefined },
): string {
  const { top, runtime } = options;
  const width = terminalWidth();
  const loads = moduleLoads(builds).filter(
    (m) => runtime === undefined || m.kind === runtime,
  );
  const out = [
    wrapItems(
      [
        bold("Modules"),
        dim(plural(loads.length, "module")),
        dim("sized by code"),
      ],
      dim(" · "),
      width,
    ),
  ];
  if (loads.length > 0) {
    const hidden = loads.length - Math.min(loads.length, top);
    out.push(
      "",
      modulesTable(builds, top, width, runtime),
      ...(hidden > 0 ? [dim(`… ${hidden} more (use --top)`)] : []),
      "",
      dim(
        prose(
          "Run `npx @convex-dev/bundle-analyzer modules <module>` to see what a module loads.",
          width,
        ),
      ),
    );
  }
  return out.join("\n");
}

/** Each module with its lists cut to `top` entries, as the report shows them. */
export function lookupJson(found: ModuleDetail[], top: number) {
  return found.map((d) => ({
    ...d,
    loadedByCount: d.loadedBy.length,
    lazilyLoadedByCount: d.lazilyLoadedBy.length,
    files: d.files.slice(0, top),
    imports: d.imports.slice(0, top),
    lazy: d.lazy.slice(0, top),
    loadedBy: d.loadedBy.slice(0, top),
    lazilyLoadedBy: d.lazilyLoadedBy.slice(0, top),
  }));
}

export function lookupReport(
  found: ModuleDetail[],
  query: string,
  top: number,
): string {
  const width = terminalWidth();
  if (found.length === 0) {
    return dim(prose(`No function module or chunk matches ${query}.`, width));
  }
  const out = found.flatMap((d) => moduleSection(d, top, width));
  if (found.some((d) => d.imports.length + d.lazy.length > 0)) {
    out.push(
      "",
      dim(
        prose(
          "Run `npx @convex-dev/bundle-analyzer modules <module>` to see any module listed here.",
          width,
        ),
      ),
    );
  }
  return out.join("\n").trimStart();
}

function moduleSection(d: ModuleDetail, top: number, width: number): string[] {
  const out = [
    heading(`${d.path} · ${d.buildId}`).trimEnd(),
    dim(
      wrapItems(
        [
          d.role,
          formatBytes(d.codeBytes),
          ...(d.imports.length > 0 || d.role === "function module"
            ? [
                `imports ${formatBytes(d.loadBytes - d.codeBytes)}`,
                `loads ${formatBytes(d.loadBytes)}`,
              ]
            : []),
          ...(d.loadedBy.length > 0
            ? [`imported by ${plural(d.loadedBy.length, "module")}`]
            : []),
          ...(d.lazilyLoadedBy.length > 0
            ? [
                `lazily imported by ${plural(d.lazilyLoadedBy.length, "module")}`,
              ]
            : []),
        ],
        " · ",
        width,
      ),
    ),
  ];
  if (d.hoisted !== null) {
    out.push(
      "",
      dim(
        prose(
          `Its source, ${d.hoisted.source}, is hoisted into ${d.hoisted.chunk}: other modules import it too, so esbuild moved it into a shared chunk and left only glue code here. Run \`npx @convex-dev/bundle-analyzer modules ${d.hoisted.chunk}\` to see its files.`,
          width,
        ),
      ),
    );
  }
  if (d.files.length > 0) {
    out.push(
      "",
      ...limitedTable(
        [
          { header: "Bundled files", flexible: true },
          { header: "Size", align: "right" },
          { header: "", optional: true },
        ],
        waterfall(d.files, (f) => f.bytes).map(([f, before]) => {
          const color = ownerColor(f.origin, f.packageName ?? f.owner);
          return [
            colored(color, f.display),
            formatBytes(f.bytes),
            waterfallBar(before, f.bytes, d.codeBytes, cells(width), color),
          ];
        }),
        top,
        width,
      ),
    );
  }
  if (d.imports.length > 0) {
    out.push("", ...importsTable("Imports", d.imports, top, width));
  } else if (d.role === "function module") {
    out.push("", dim("It imports no other modules."));
  }
  if (d.lazy.length > 0) {
    out.push(
      "",
      dim(prose(`Lazy imports are only loaded ${LAZY_NOTES[d.kind]}.`, width)),
      "",
      ...importsTable("Lazy imports", d.lazy, top, width),
    );
  }
  if (d.loadedBy.length > 0) {
    out.push("", ...importerTable(d, d.loadedBy, false, top, width));
  }
  if (d.lazilyLoadedBy.length > 0) {
    out.push("", ...importerTable(d, d.lazilyLoadedBy, true, top, width));
  }
  return out;
}

function limitedTable(
  columns: Parameters<typeof table>[0],
  rows: string[][],
  top: number,
  width: number,
): string[] {
  const hidden = rows.length - Math.min(rows.length, top);
  return [
    table(columns, rows.slice(0, top), width),
    ...(hidden > 0 ? [dim(`… ${hidden} more (use --top)`)] : []),
  ];
}

/**
 * The function modules that import `d`, with what each loads when it uses
 * `d` (see `ModuleLoader.totalBytes`) and how much of that is `d`.
 */
function importerTable(
  d: ModuleDetail,
  importers: ModuleLoader[],
  lazy: boolean,
  top: number,
  width: number,
): string[] {
  const header = lazy ? "Lazily imported by" : "Imported by";
  const totals = importers.map((l) => l.totalBytes);
  const max = Math.max(1, ...totals);
  const cells = barCells(width, 50, 16);
  return limitedTable(
    [
      { header, flexible: true },
      { header: "Total", align: "right" },
      { header: "Share", align: "right" },
      {
        header: legend(
          [
            { label: "this module", color: CODE_COLOR },
            { label: "rest", color: SHARED_CHUNK_COLOR },
          ],
          Number.POSITIVE_INFINITY,
        ),
        optional: true,
      },
    ],
    importers.map((l, idx) => [
      l.path,
      formatBytes(totals[idx]),
      formatPercent(d.codeBytes / totals[idx]),
      stackedBar(
        [
          { value: d.codeBytes, color: CODE_COLOR },
          { value: totals[idx] - d.codeBytes, color: SHARED_CHUNK_COLOR },
        ],
        Math.max(1, Math.round((totals[idx] / max) * cells)),
      ),
    ]),
    top,
    width,
  );
}

function importsTable(
  header: string,
  modules: ImportedModule[],
  top: number,
  width: number,
): string[] {
  const total = modules.reduce((n, m) => n + m.codeBytes, 0);
  return limitedTable(
    [
      { header, flexible: true },
      { header: "Size", align: "right" },
      { header: "", optional: true },
    ],
    waterfall(modules, (m) => m.codeBytes).map(([m, before]) => [
      m.path,
      formatBytes(m.codeBytes),
      waterfallBar(before, m.codeBytes, total, cells(width), CODE_COLOR),
    ]),
    top,
    width,
  );
}

/** Each item with the sum of the items before it, for a waterfall bar. */
function waterfall<T>(items: T[], size: (item: T) => number): [T, number][] {
  let before = 0;
  return items.map((item) => {
    const step: [T, number] = [item, before];
    before += size(item);
    return step;
  });
}

function cells(width: number): number {
  return barCells(width, 40, 24);
}

export function modulesJson(
  builds: AnalyzedBuild[],
  limit: number,
  runtime?: FunctionRuntime,
) {
  return moduleLoads(builds)
    .filter((m) => runtime === undefined || m.kind === runtime)
    .slice(0, limit);
}
