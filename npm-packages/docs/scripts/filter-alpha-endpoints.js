#!/usr/bin/env node
const fs = require("fs");
const path = require("path");

const HTTP_METHODS = [
  "get",
  "put",
  "post",
  "delete",
  "options",
  "head",
  "patch",
  "trace",
];
const ALPHA_TAG = "alpha";

const SOURCES = [
  "../@convex-dev/platform/management-openapi.json",
  "../@convex-dev/platform/public-deployment-openapi.json",
  "../@convex-dev/platform/deployment-openapi.json",
];

const repoRoot = path.resolve(__dirname, "..");
const outDir = path.join(repoRoot, ".openapi-filtered");
fs.mkdirSync(outDir, { recursive: true });

for (const relPath of SOURCES) {
  const srcPath = path.join(repoRoot, relPath);
  const spec = JSON.parse(fs.readFileSync(srcPath, "utf8"));

  for (const [pathStr, pathItem] of Object.entries(spec.paths ?? {})) {
    for (const method of HTTP_METHODS) {
      const op = pathItem[method];
      if (op?.tags?.includes(ALPHA_TAG)) {
        delete pathItem[method];
      } else if (op?.tags) {
        op.tags = op.tags.filter((t) => t !== ALPHA_TAG);
      }
    }
    const hasOperation = HTTP_METHODS.some((m) => pathItem[m] !== undefined);
    if (!hasOperation) {
      delete spec.paths[pathStr];
    }
  }

  if (Array.isArray(spec.tags)) {
    spec.tags = spec.tags.filter((t) => t.name !== ALPHA_TAG);
  }

  const outPath = path.join(outDir, path.basename(relPath));
  fs.writeFileSync(outPath, JSON.stringify(spec, null, 2));
  console.log(`✓ Filtered ${relPath} → ${path.relative(repoRoot, outPath)}`);
}
