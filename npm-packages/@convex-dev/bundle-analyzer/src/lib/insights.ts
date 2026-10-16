import { AnalyzedBuild, BuildKind, inputBytes, ownerLabel } from "./model.js";
import {
  bold,
  colored,
  dim,
  formatBytes,
  terminalWidth,
  wrapItems,
} from "./format.js";
import { FunctionRuntime, isFunctionRuntime, moduleLoads } from "./modules.js";
import { packageUsage } from "./packages.js";

const LEVEL_ICONS: Record<Insight["level"], [string, string]> = {
  warn: ["#f3b01c", "▲"],
  info: ["#4fb3d9", "●"],
};

export function insightsReport(
  found: Insight[],
  width = terminalWidth(),
): string {
  return found
    .map((i) => {
      const [color, icon] = LEVEL_ICONS[i.level];
      const title = wrapItems(
        i.title.split(" "),
        " ",
        width,
        "    ",
      ).trimStart();
      const detail = wrapItems(i.detail.split(" "), " ", width, "    ");
      return `  ${colored(color, icon)} ${bold(title)}\n${dim(detail)}`;
    })
    .join("\n");
}

export type InsightKind =
  | "duplicate-package"
  | "use-node-file-in-convex-runtime"
  | "dev-dependency-bundled"
  | "test-file-bundled"
  | "large-node-package"
  | "large-convex-runtime-package"
  | "package-in-many-components"
  | "heavy-module";

export type Insight = {
  kind: InsightKind;
  level: "warn" | "info";
  title: string;
  detail: string;
  /** The size the insight is about, in bytes. */
  bytes: number;
  buildIds: string[];
  /** The package (`name@version`) or module path the insight is about. */
  subject: string | null;
};

// Absolute sizes: deployment limits are enforced (and can change) server side.
const DUPLICATE_MIN_EXTRA_BYTES = 10_000;
const LARGE_PACKAGE_BYTES = 500_000;
const SHARED_PACKAGE_MIN_BYTES = 50_000;
const SHARED_PACKAGE_MIN_COMPONENTS = 3;
const HEAVY_MODULE_LOAD_BYTES = 5_000_000;
const MAX_HEAVY_MODULES = 5;

/**
 * Suggestions for shrinking the bundle. Duplicate sizes count code plus source
 * maps, which are uploaded and stored together. Package and module load sizes
 * count code, which is what's compiled and evaluated.
 * `devDependencies` are the names in the project's package.json.
 * Warnings come first, then each kind in a fixed order, largest first.
 */
export function findInsights(
  builds: AnalyzedBuild[],
  devDependencies: ReadonlySet<string> = new Set(),
): Insight[] {
  const found = [
    ...duplicateInsights(builds),
    ...nodeFileInsights(builds),
    ...heavyModuleInsights(builds),
    ...devCodeInsights(builds, devDependencies),
    ...packageInsights(builds),
  ];
  const order = { warn: 0, info: 1 };
  return found.sort((a, b) => order[a.level] - order[b.level]);
}

function duplicateInsights(builds: AnalyzedBuild[]) {
  const result: Insight[] = [];
  for (const d of duplicates(builds)) {
    const extra = d.copies.slice(1).reduce((n, c) => n + c.bytes, 0);
    if (extra < DUPLICATE_MIN_EXTRA_BYTES) continue;
    const sameVersion =
      new Set(d.copies.map((c) => c.version)).size < d.copies.length;
    const copies = d.copies
      .map((c) => {
        const where = sameVersion
          ? ` in ${c.packageDir.replace(/^(\.\.\/)+/, "")}`
          : "";
        return `${c.version ?? "?"}${where} (${formatBytes(c.bytes)})`;
      })
      .join(", ");
    result.push({
      kind: "duplicate-package",
      level: "warn",
      title: `${d.name} is bundled ${d.copies.length} times in ${d.buildId}`,
      detail: `Copies: ${copies}. Aligning the versions your dependencies need, or deduplicating (e.g. \`npm dedupe\`), would save ${formatBytes(extra)}. \`npx @convex-dev/bundle-analyzer packages ${d.name}\` shows what imports each copy.`,
      bytes: extra,
      buildIds: [d.buildId],
      subject: d.name,
    });
  }
  return result.sort((a, b) => b.bytes - a.bytes);
}

const HEAVY_MODULE_COST: Record<FunctionRuntime, string> = {
  convex:
    "Every call to a function in it compiles and evaluates this much code.",
  node: "A Node instance evaluates this much bundled code, plus its external packages, the first time it runs a function in it.",
};

const HEAVY_MODULE_FIX: Record<FunctionRuntime, string> = {
  convex:
    "Moving rarely used dependencies into another module keeps them off this path. Actions can also load them with `await import()`; queries and mutations can't.",
  node: "Moving rarely used dependencies behind `await import()` or into another module keeps them off this path.",
};

const MAX_HEAVY_MODULE_OWNERS = 3;

function heavyModuleInsights(builds: AnalyzedBuild[]): Insight[] {
  const byId = new Map(builds.map((b) => [b.id, b]));
  return moduleLoads(builds)
    .filter((m) => m.loadBytes >= HEAVY_MODULE_LOAD_BYTES)
    .slice(0, MAX_HEAVY_MODULES)
    .map((m) => ({
      kind: "heavy-module",
      level: "warn",
      title: `${m.path} (${m.buildId}) loads ${formatBytes(m.loadBytes)} of code`,
      detail: `${HEAVY_MODULE_COST[m.kind as FunctionRuntime]} Largest contributors: ${loadedOwners(
        byId.get(m.buildId)!,
        [m.path, ...m.chunks],
      )
        .map((o) => `${o.owner} ${formatBytes(o.bytes)}`)
        .join(
          ", ",
        )}. ${HEAVY_MODULE_FIX[m.kind as FunctionRuntime]} \`npx @convex-dev/bundle-analyzer modules ${m.path}\` shows what it loads.`,
      bytes: m.loadBytes,
      buildIds: [m.buildId],
      subject: m.path,
    }));
}

/** The largest owners of code in `paths`, a module and the chunks it loads. */
function loadedOwners(
  b: AnalyzedBuild,
  paths: string[],
): { owner: string; bytes: number }[] {
  const wanted = new Set(paths);
  const owners = new Map<string, number>();
  for (const o of b.outputs) {
    if (!wanted.has(o.path)) continue;
    for (const i of o.inputs) {
      const k = ownerLabel(i);
      owners.set(k, (owners.get(k) ?? 0) + i.bytesInOutput);
    }
  }
  return [...owners.entries()]
    .map(([owner, bytes]) => ({ owner, bytes }))
    .sort((x, y) => y.bytes - x.bytes)
    .slice(0, MAX_HEAVY_MODULE_OWNERS);
}

/**
 * "use node" files are the node build's entry points. The Convex runtime build
 * of the same component only bundles one when a file without "use node"
 * imports it.
 */
function nodeFileInsights(builds: AnalyzedBuild[]): Insight[] {
  const result: Insight[] = [];
  for (const node of builds) {
    if (node.kind !== "node") continue;
    const useNodeFiles = new Set(
      node.outputs.flatMap((o) => (o.entryPoint ? [o.entryPoint] : [])),
    );
    for (const b of builds) {
      if (
        b.kind !== "convex" ||
        b.component.definitionPath !== node.component.definitionPath
      ) {
        continue;
      }
      const code = new Map<string, number>();
      for (const o of b.outputs) {
        for (const i of o.inputs) {
          if (i.bytesInOutput > 0 && useNodeFiles.has(i.path)) {
            code.set(i.path, (code.get(i.path) ?? 0) + i.bytesInOutput);
          }
        }
      }
      for (const [path, bytes] of code) {
        result.push({
          kind: "use-node-file-in-convex-runtime",
          level: "warn",
          title: `${path} has "use node" but is bundled into ${b.id}`,
          detail: `A file without "use node" imports it, so it and everything it imports are bundled for the Convex runtime too. Moving what both need into a file without "use node" keeps Node-only code out. \`npx @convex-dev/bundle-analyzer files ${path}\` shows what imports it.`,
          bytes,
          buildIds: [b.id],
          subject: path,
        });
      }
    }
  }
  return result.sort((a, b) => b.bytes - a.bytes);
}

const DEV_DEPENDENCY_MIN_BYTES = 10_000;
const TEST_FILE = /(^|\/)(__tests__\/|[^/]+\.(test|spec)\.[cm]?[jt]sx?$)/;

/** devDependencies and test files that deployed functions import. */
function devCodeInsights(
  builds: AnalyzedBuild[],
  devDependencies: ReadonlySet<string>,
): Insight[] {
  const packages: Insight[] = [];
  const tests: Insight[] = [];
  const deployed = builds.filter((b) => isFunctionRuntime(b.kind));
  for (const pkg of packageUsage(deployed)) {
    if (pkg.origin !== "npm" || !devDependencies.has(pkg.name)) continue;
    if (pkg.codeBytes < DEV_DEPENDENCY_MIN_BYTES) continue;
    const buildIds = [...pkg.builds.keys()];
    packages.push({
      kind: "dev-dependency-bundled",
      level: "info",
      title: `${pkg.key}, a devDependency, is bundled into ${buildIds.join(", ")}`,
      detail: `Deployed functions import it, adding ${formatBytes(pkg.codeBytes)} of code. If only tests or scripts need it, \`npx @convex-dev/bundle-analyzer packages ${pkg.name}\` shows which files import it; if functions need it, it belongs in dependencies.`,
      bytes: pkg.codeBytes,
      buildIds,
      subject: pkg.key,
    });
  }
  const testFiles = new Map<string, { bytes: number; buildIds: Set<string> }>();
  for (const b of deployed) {
    for (const o of b.outputs) {
      for (const i of o.inputs) {
        if (i.origin !== "app" || i.bytesInOutput === 0) continue;
        if (!TEST_FILE.test(i.path)) continue;
        const entry = testFiles.get(i.path) ?? {
          bytes: 0,
          buildIds: new Set(),
        };
        entry.bytes += i.bytesInOutput;
        entry.buildIds.add(b.id);
        testFiles.set(i.path, entry);
      }
    }
  }
  for (const [path, { bytes, buildIds }] of testFiles) {
    tests.push({
      kind: "test-file-bundled",
      level: "info",
      title: `Test file ${path} is bundled into ${[...buildIds].join(", ")}`,
      detail: `A deployed function imports it, along with anything it imports. \`npx @convex-dev/bundle-analyzer files ${path}\` shows what imports it.`,
      bytes,
      buildIds: [...buildIds],
      subject: path,
    });
  }
  return [packages, tests].flatMap((list) =>
    list.sort((a, b) => b.bytes - a.bytes),
  );
}

function packageInsights(builds: AnalyzedBuild[]): Insight[] {
  const byId = new Map(builds.map((b) => [b.id, b]));
  const node: Insight[] = [];
  const convex: Insight[] = [];
  const shared: Insight[] = [];
  const codeByPackage = packageCodeByBuild(builds);
  for (const pkg of packageUsage(builds)) {
    if (pkg.origin !== "npm") continue;
    const code = codeByPackage.get(pkg.key)!;
    // Builds where the package has code, leaving out ones where it only
    // contributes source maps.
    const withCode = [...code.keys()].filter((id) => code.get(id)! > 0);
    const ids = (kind: BuildKind) =>
      withCode.filter((id) => byId.get(id)?.kind === kind);
    const sum = (list: string[]) =>
      list.reduce((n, id) => n + (code.get(id) ?? 0), 0);

    const nodeIds = ids("node");
    const nodeBytes = sum(nodeIds);
    if (nodeBytes >= LARGE_PACKAGE_BYTES) {
      node.push({
        kind: "large-node-package",
        level: "info",
        title: `${pkg.key} adds ${formatBytes(nodeBytes)} to "use node" bundles`,
        detail: `Adding "${pkg.name}" to \`node.externalPackages\` in convex.json installs it on the server instead of bundling it.`,
        bytes: nodeBytes,
        buildIds: nodeIds,
        subject: pkg.key,
      });
    }

    const convexIds = ids("convex");
    const convexBytes = sum(convexIds);
    if (convexBytes >= LARGE_PACKAGE_BYTES) {
      convex.push({
        kind: "large-convex-runtime-package",
        level: "info",
        title: `${pkg.key} adds ${formatBytes(convexBytes)} to the Convex runtime bundle`,
        detail: `Run \`npx @convex-dev/bundle-analyzer packages ${pkg.name}\` to see which of your files import it.`,
        bytes: convexBytes,
        buildIds: convexIds,
        subject: pkg.key,
      });
    }

    const components = new Set(
      withCode.map((id) => byId.get(id)?.component.label),
    );
    if (
      components.size >= SHARED_PACKAGE_MIN_COMPONENTS &&
      pkg.codeBytes >= SHARED_PACKAGE_MIN_BYTES
    ) {
      shared.push({
        kind: "package-in-many-components",
        level: "info",
        title: `${pkg.key} is bundled separately into ${components.size} components`,
        detail: `Each component is bundled on its own, so a dependency they share is included once per component (${formatBytes(pkg.codeBytes)} of code in total).`,
        bytes: pkg.codeBytes,
        buildIds: withCode,
        subject: pkg.key,
      });
    }
  }
  return [node, convex, shared].flatMap((list) =>
    list.sort((a, b) => b.bytes - a.bytes),
  );
}

/** Code bytes of each package, keyed by `ownerLabel`, in each build by id. */
function packageCodeByBuild(
  builds: AnalyzedBuild[],
): Map<string, Map<string, number>> {
  const result = new Map<string, Map<string, number>>();
  for (const b of builds) {
    for (const o of b.outputs) {
      for (const i of o.inputs) {
        const key = ownerLabel(i);
        let byBuild = result.get(key);
        if (!byBuild) {
          byBuild = new Map();
          result.set(key, byBuild);
        }
        byBuild.set(b.id, (byBuild.get(b.id) ?? 0) + i.bytesInOutput);
      }
    }
  }
  return result;
}

export type Duplicate = {
  name: string;
  /** Distinct installations (package dirs) of the package in a single build. */
  copies: { version: string | null; packageDir: string; bytes: number }[];
  buildId: string;
};

export function duplicates(builds: AnalyzedBuild[]): Duplicate[] {
  const result: Duplicate[] = [];
  for (const b of builds) {
    const byName = new Map<
      string,
      Map<string, { version: string | null; bytes: number }>
    >();
    for (const o of b.outputs) {
      for (const i of o.inputs) {
        if (!i.packageName || !i.packageDir) continue;
        let dirs = byName.get(i.packageName);
        if (!dirs) {
          dirs = new Map();
          byName.set(i.packageName, dirs);
        }
        const entry = dirs.get(i.packageDir) ?? {
          version: i.packageVersion,
          bytes: 0,
        };
        entry.bytes += inputBytes(i);
        dirs.set(i.packageDir, entry);
      }
    }
    for (const [name, dirs] of byName) {
      if (dirs.size < 2) continue;
      result.push({
        name,
        buildId: b.id,
        copies: [...dirs.entries()]
          .map(([packageDir, v]) => ({ packageDir, ...v }))
          .sort((x, y) => y.bytes - x.bytes),
      });
    }
  }
  return result;
}
