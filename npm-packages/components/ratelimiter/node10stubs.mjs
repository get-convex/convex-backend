/* eslint-disable */
import fs from "fs/promises";
import path from "path";

async function findPackageJson(directory) {
  const packagePath = path.join(directory, "package.json");
  try {
    await fs.access(packagePath);
    return packagePath;
  } catch (error) {
    const parentDir = path.dirname(directory);
    if (parentDir === directory) {
      throw new Error("package.json not found");
    }
    return findPackageJson(parentDir);
  }
}

async function processSubPackages(packageJsonPath, exports, cleanup = false) {
  const baseDir = path.dirname(packageJsonPath);

  for (const [subDir, _] of Object.entries(exports)) {
    // package.json is already right where Node10 resolution would expect it.
    if (subDir.endsWith("package.json")) continue;
    // No need for Node10 resolution for component.config.ts
    if (subDir.endsWith("convex.config.js")) continue;
    // . just works with Node10 resolution
    if (subDir === ".") continue;
    console.log(subDir);

    const newDir = path.join(baseDir, subDir);
    const newPackageJsonPath = path.join(newDir, "package.json");

    if (cleanup) {
      try {
        await fs.rm(newDir, { recursive: true, force: true });
      } catch (error) {
        console.error(`Failed to remove ${newDir}:`, error.message);
      }
    } else {
      const newPackageJson = {
        main: `../dist/commonjs/${subDir}/index.js`,
        module: `../dist/esm/${subDir}/index.js`,
        types: `../dist/commonjs/${subDir}/index.d.ts`,
      };

      await fs.mkdir(newDir, { recursive: true });
      await fs.writeFile(
        newPackageJsonPath,
        JSON.stringify(newPackageJson, null, 2),
      );
    }
  }
}

async function main() {
  try {
    const isCleanup = process.argv.includes("--cleanup");
    const isAddFiles = process.argv.includes("--addFiles");
    const packageJsonPath = await findPackageJson(process.cwd());
    const packageJson = JSON.parse(await fs.readFile(packageJsonPath, "utf-8"));

    if (!packageJson.exports) {
      throw new Error("exports not found in package.json");
    }

    if (isAddFiles) {
      return;
    }

    await processSubPackages(packageJsonPath, packageJson.exports, isCleanup);

    if (isCleanup) {
      console.log(
        "Node10 module resolution compatibility stub directories removed.",
      );
    } else {
      console.log(
        "Node10 module resolution compatibility stub directories created",
      );
    }
  } catch (error) {
    console.error("Error:", error.message);
  }
}

main();
