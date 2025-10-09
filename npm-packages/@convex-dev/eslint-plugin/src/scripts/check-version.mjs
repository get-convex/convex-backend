#!/usr/bin/env node

import { readFileSync } from "fs";
import { fileURLToPath } from "url";
import { dirname, join } from "path";

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

// Read package.json version
const packageJsonPath = join(__dirname, "../../package.json");
const packageJson = JSON.parse(readFileSync(packageJsonPath, "utf-8"));
const packageVersion = packageJson.version;

// Read version.ts version
const versionTsPath = join(__dirname, "../version.ts");
const versionTsContent = readFileSync(versionTsPath, "utf-8");
const versionMatch = versionTsContent.match(
  /^export const version = "(.+)";\n$/,
);

if (!versionMatch) {
  console.error("Error: Could not parse version from version.ts");
  process.exit(1);
}

const tsVersion = versionMatch[1];

// Compare versions
if (packageVersion !== tsVersion) {
  console.error(
    `Version mismatch!\n` +
      `  package.json: ${packageVersion}\n` +
      `  version.ts:   ${tsVersion}\n` +
      `\nPlease update version.ts to match package.json before publishing.`,
  );
  process.exit(1);
}

console.log(
  `✓ The version in version.ts (${packageVersion}) matches package.json`,
);
