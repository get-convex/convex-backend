import {
  AnalyzedBuild,
  SourceOrigin,
  inputBytes,
  ownerLabel,
} from "./model.js";
import {
  bar,
  barCells,
  colored,
  dim,
  formatBytes,
  formatPercent,
  ownerColor,
  table,
  terminalWidth,
} from "./format.js";

export function packagesTable(
  builds: AnalyzedBuild[],
  top: number,
  width = terminalWidth(),
): string {
  const all = packageUsage(builds).filter((p) => p.bytes > 0);
  const grand = Math.max(
    1,
    all.reduce((n, p) => n + p.bytes, 0),
  );
  const max = Math.max(1, ...all.map((p) => p.bytes));
  const cells = barCells(width, 70, 16);
  const rows = all.slice(0, top).map((p, idx) => {
    const color = ownerColor(p.origin, p.name);
    return [
      dim(String(idx + 1)),
      colored(color, p.key),
      formatBytes(p.bytes),
      dim(formatBytes(p.codeBytes)),
      dim(formatBytes(p.sourceMapBytes)),
      formatPercent(p.bytes / grand),
      String(p.files),
      bar(p.bytes / max, cells, color),
    ];
  });
  const hidden = all.length - rows.length;
  return (
    table(
      [
        { header: "#", align: "right" },
        { header: "Package", flexible: true, truncate: "end" },
        { header: "Size", align: "right" },
        { header: "Code", align: "right", optional: true },
        { header: "Source maps", align: "right", optional: true },
        { header: "Share", align: "right" },
        { header: "Files", align: "right" },
        { header: "", optional: true },
      ],
      rows,
      width,
    ) + (hidden > 0 ? `\n${dim(`… ${hidden} more (use --top)`)}` : "")
  );
}

export function packagesJson(builds: AnalyzedBuild[], limit: number) {
  return packageUsage(builds)
    .filter((p) => p.bytes > 0)
    .slice(0, limit)
    .map((p) => ({
      package: p.key,
      name: p.name,
      version: p.version,
      origin: p.origin,
      bytes: p.bytes,
      codeBytes: p.codeBytes,
      sourceMapBytes: p.sourceMapBytes,
      files: p.files,
      bytesByBuild: Object.fromEntries(p.builds),
      packageDirs: [...p.packageDirs],
    }));
}

export type PackageUsage = {
  key: string;
  name: string;
  version: string | null;
  origin: SourceOrigin;
  /** Code plus source map bytes. */
  bytes: number;
  codeBytes: number;
  sourceMapBytes: number;
  files: number;
  /** `bytes` in each build the package appears in, by build id. */
  builds: Map<string, number>;
  packageDirs: Set<string>;
};

export function packageUsage(builds: AnalyzedBuild[]): PackageUsage[] {
  const byKey = new Map<string, PackageUsage>();
  for (const b of builds) {
    for (const o of b.outputs) {
      for (const i of o.inputs) {
        const key = ownerLabel(i);
        let usage = byKey.get(key);
        if (!usage) {
          usage = {
            key,
            name: i.packageName ?? key,
            version: i.packageVersion,
            origin: i.origin,
            bytes: 0,
            codeBytes: 0,
            sourceMapBytes: 0,
            files: 0,
            builds: new Map(),
            packageDirs: new Set(),
          };
          byKey.set(key, usage);
        }
        const bytes = inputBytes(i);
        usage.bytes += bytes;
        usage.codeBytes += i.bytesInOutput;
        usage.sourceMapBytes += i.sourceMapBytes;
        usage.files += 1;
        usage.builds.set(b.id, (usage.builds.get(b.id) ?? 0) + bytes);
        if (i.packageDir) usage.packageDirs.add(i.packageDir);
      }
    }
  }
  return [...byKey.values()].sort((a, b) => b.bytes - a.bytes);
}
