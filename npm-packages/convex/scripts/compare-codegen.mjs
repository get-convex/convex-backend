#!/usr/bin/env node
/**
 * Script to compare directly-generated JS/DTS files with TS-compiled outputs.
 *
 * This creates two directories with the different codegen approaches and
 * automatically opens the diffs in VS Code.
 */

import { apiCodegen } from "../dist/esm/cli/codegen_templates/api.js";
import { serverCodegen } from "../dist/esm/cli/codegen_templates/server.js";
import { noSchemaDataModelDTS } from "../dist/esm/cli/codegen_templates/dataModel.js";
import * as fs from "fs";
import * as path from "path";
import * as os from "os";
import { execSync } from "child_process";
import prettier from "prettier";

const modulePaths = ["foo.ts", "bar/baz.ts"];

console.log("Generating codegen outputs for comparison...\n");

// Generate all versions of our files
const apiJsDts = apiCodegen(modulePaths, { generateJavaScriptApi: true });
const apiJs = apiJsDts.JS;
const apiDts = apiJsDts.DTS;
const apiTsResult = apiCodegen(modulePaths, { generateJavaScriptApi: false });
const apiTs = apiTsResult.TS;

const serverJsDts = serverCodegen({ generateJavaScriptApi: true });
const serverJs = serverJsDts.JS;
const serverDts = serverJsDts.DTS;
const serverTsResult = serverCodegen({ generateJavaScriptApi: false });
const serverTs = serverTsResult.TS;

const dataModelDts = noSchemaDataModelDTS();

const tmpDir = fs.mkdtempSync(
  path.join(os.tmpdir(), "convex-codegen-compare-"),
);

try {
  // Create directory structure
  const directDir = path.join(tmpDir, "direct");
  const compiledDir = path.join(tmpDir, "compiled");
  const tsSourceDir = path.join(tmpDir, "ts_source");

  fs.mkdirSync(directDir, { recursive: true });
  fs.mkdirSync(compiledDir, { recursive: true });
  fs.mkdirSync(tsSourceDir);

  // Write directly-generated JS/DTS files (formatted with prettier)
  const formattedApiJs = await prettier.format(apiJs, {
    parser: "typescript",
    pluginSearchDirs: false,
  });
  const formattedApiDts = await prettier.format(apiDts, {
    parser: "typescript",
    pluginSearchDirs: false,
  });
  const formattedServerJs = await prettier.format(serverJs, {
    parser: "typescript",
    pluginSearchDirs: false,
  });
  const formattedServerDts = await prettier.format(serverDts, {
    parser: "typescript",
    pluginSearchDirs: false,
  });
  const formattedDataModelDts = await prettier.format(dataModelDts, {
    parser: "typescript",
    pluginSearchDirs: false,
  });

  fs.writeFileSync(path.join(directDir, "api.js"), formattedApiJs);
  fs.writeFileSync(path.join(directDir, "api.d.ts"), formattedApiDts);
  fs.writeFileSync(path.join(directDir, "server.js"), formattedServerJs);
  fs.writeFileSync(path.join(directDir, "server.d.ts"), formattedServerDts);
  fs.writeFileSync(
    path.join(directDir, "dataModel.d.ts"),
    formattedDataModelDts,
  );

  // Format TS files with prettier BEFORE compilation (matching runtime behavior)
  const formattedApiTs = await prettier.format(apiTs, {
    parser: "typescript",
    pluginSearchDirs: false,
  });
  const formattedServerTs = await prettier.format(serverTs, {
    parser: "typescript",
    pluginSearchDirs: false,
  });
  const formattedDataModelTs = await prettier.format(dataModelDts, {
    parser: "typescript",
    pluginSearchDirs: false,
  });

  // Write the formatted TS files we'll compile
  fs.writeFileSync(path.join(tsSourceDir, "api.ts"), formattedApiTs);
  fs.writeFileSync(path.join(tsSourceDir, "server.ts"), formattedServerTs);
  fs.writeFileSync(
    path.join(tsSourceDir, "dataModel.ts"),
    formattedDataModelTs,
  );

  // Create tsconfig.json for compilation
  const tsconfig = {
    compilerOptions: {
      target: "ES2020",
      module: "ES2020",
      declaration: true,
      emitDeclarationOnly: false,
      skipLibCheck: true,
      esModuleInterop: true,
      moduleResolution: "bundler",
      outDir: compiledDir,
      rootDir: tsSourceDir,
      paths: {
        "convex/server": [
          path.join(
            tmpDir,
            "node_modules/convex/dist/esm-types/server/index.d.ts",
          ),
        ],
        "convex/values": [
          path.join(
            tmpDir,
            "node_modules/convex/dist/esm-types/values/index.d.ts",
          ),
        ],
      },
    },
    include: [path.join(tsSourceDir, "*.ts")],
  };
  fs.writeFileSync(
    path.join(tmpDir, "tsconfig.json"),
    JSON.stringify(tsconfig, null, 2),
  );

  // Set up the convex package
  const convexPackageDir = path.join(tmpDir, "node_modules", "convex");
  const convexDistDir = path.join(convexPackageDir, "dist");
  fs.mkdirSync(convexDistDir, { recursive: true });

  const projectRoot = process.cwd();
  const distDir = path.join(projectRoot, "dist");

  // Copy package.json
  fs.copyFileSync(
    path.join(projectRoot, "package.json"),
    path.join(convexPackageDir, "package.json"),
  );

  // Symlink dist directories
  fs.symlinkSync(path.join(distDir, "esm"), path.join(convexDistDir, "esm"));
  fs.symlinkSync(
    path.join(distDir, "esm-types"),
    path.join(convexDistDir, "esm-types"),
  );

  // Create stub user modules that are imported by api.ts
  for (const modulePath of modulePaths) {
    const fullPath = path.join(tmpDir, modulePath);
    const dir = path.dirname(fullPath);
    fs.mkdirSync(dir, { recursive: true });
    fs.writeFileSync(fullPath.replace(/\.ts$/, ".js"), "export const foo = 1;");
    fs.writeFileSync(
      fullPath.replace(/\.ts$/, ".d.ts"),
      "export declare const foo: number;",
    );
  }

  // Compile the TS files
  const tscPath = path.join(projectRoot, "node_modules", ".bin", "tsc");
  try {
    execSync(`${tscPath} --project ${path.join(tmpDir, "tsconfig.json")}`, {
      cwd: tmpDir,
      stdio: "pipe",
    });
  } catch (error) {
    console.error("TypeScript compilation failed:");
    console.error(error.stdout?.toString());
    console.error(error.stderr?.toString());
    process.exit(1);
  }

  // Format the compiled outputs with prettier
  for (const file of [
    "api.js",
    "api.d.ts",
    "server.js",
    "server.d.ts",
    "dataModel.d.ts",
  ]) {
    const filePath = path.join(compiledDir, file);
    const content = fs.readFileSync(filePath, "utf-8");
    const formatted = await prettier.format(content, {
      parser: "typescript",
      pluginSearchDirs: false,
    });
    fs.writeFileSync(filePath, formatted);
  }

  console.log("✓ Generated codegen outputs\n");
  console.log("Directories created:");
  console.log(`  Direct (JS/DTS):  ${directDir}`);
  console.log(`  Compiled (TS):    ${compiledDir}`);
  console.log(`  TS Source:        ${tsSourceDir}`);
  console.log();
  console.log("To compare with diff:");
  console.log(`  diff -r ${directDir} ${compiledDir}`);
  console.log();
  console.log("To open diffs in VS Code:");
  const filesToCompare = [
    "api.d.ts",
    "api.js",
    "server.d.ts",
    "server.js",
    "dataModel.d.ts",
  ];
  for (const file of filesToCompare) {
    console.log(`  code --diff ${directDir}/${file} ${compiledDir}/${file}`);
  }
  console.log();
} catch (error) {
  console.error("Error:", error);
  // Clean up on error
  fs.rmSync(tmpDir, { recursive: true, force: true });
  process.exit(1);
}
