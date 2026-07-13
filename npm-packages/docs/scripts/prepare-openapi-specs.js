#!/usr/bin/env node
// Prepares the platform OpenAPI specs for the docs build, writing the results
// to `.openapi-filtered/`. Two tag-driven transforms:
//   - `alpha`: the operation is dropped entirely, so it never appears in docs.
//   - `beta`: the operation is kept but a Beta admonition is prepended to its
//     description so it renders with a visible beta marker.
// Both control tags are stripped from the output so they don't leak into the
// rendered tag groups.
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
const BETA_TAG = "beta";
const CONTROL_TAGS = [ALPHA_TAG, BETA_TAG];

const BETA_NOTICE =
  ":::info[Beta]\n\n" +
  "This endpoint is in beta. Its behavior and response shape may change.\n\n" +
  ":::";

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
      if (!op?.tags) {
        continue;
      }
      if (op.tags.includes(ALPHA_TAG)) {
        delete pathItem[method];
        continue;
      }
      if (op.tags.includes(BETA_TAG)) {
        op.description = op.description
          ? `${BETA_NOTICE}\n\n${op.description}`
          : BETA_NOTICE;
      }
      // Strip control tags so they don't render as their own tag groups.
      op.tags = op.tags.filter((t) => !CONTROL_TAGS.includes(t));
    }
    const hasOperation = HTTP_METHODS.some((m) => pathItem[m] !== undefined);
    if (!hasOperation) {
      delete spec.paths[pathStr];
    }
  }

  if (Array.isArray(spec.tags)) {
    spec.tags = spec.tags.filter((t) => !CONTROL_TAGS.includes(t.name));
  }

  const outPath = path.join(outDir, path.basename(relPath));
  fs.writeFileSync(outPath, JSON.stringify(spec, null, 2));
  console.log(`✓ Prepared ${relPath} → ${path.relative(repoRoot, outPath)}`);
}
