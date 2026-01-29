import path from "path";
import { execFile } from "child_process";
import { promisify } from "util";
import { Filesystem } from "./fs.js";
import { Context } from "./context.js";
import { logVerbose, logWarning } from "./log.js";
import { chalkStderr } from "chalk";

const execFileAsync = promisify(execFile);

export interface RustBuildResult {
  path: string;
  // Base64-encoded WASM binary
  wasmBinary: string;
  // JSON metadata about exported functions
  functionMetadata: RustFunctionMetadata[];
  environment: "rust";
}

export interface RustFunctionMetadata {
  name: string;
  functionType: "query" | "mutation" | "action";
  exportName: string;
}

// Check if cargo is available
export async function checkCargoInstalled(): Promise<boolean> {
  try {
    await execFileAsync("cargo", ["--version"]);
    return true;
  } catch {
    return false;
  }
}

// Find Cargo.toml in the directory hierarchy
export async function findCargoToml(
  fs: Filesystem,
  startDir: string,
): Promise<string | null> {
  let currentDir = startDir;
  while (true) {
    const cargoPath = path.join(currentDir, "Cargo.toml");
    if (fs.exists(cargoPath)) {
      return cargoPath;
    }
    const parentDir = path.dirname(currentDir);
    if (parentDir === currentDir) {
      break;
    }
    currentDir = parentDir;
  }
  return null;
}

// Build a Rust file to WASM
export async function buildRustModule(
  ctx: Context,
  filePath: string,
): Promise<RustBuildResult> {
  logVerbose(chalkStderr.yellow(`Building Rust module: ${filePath}`));

  // Check for cargo
  const hasCargo = await checkCargoInstalled();
  if (!hasCargo) {
    return await ctx.crash({
      exitCode: 1,
      errorType: "invalid filesystem data",
      printedMessage:
        `Rust support requires Cargo to be installed.\n` +
        `Please install Rust and Cargo: https://rustup.rs/`,
    });
  }

  // Find Cargo.toml or create a temporary one
  const cargoTomlPath = await findCargoToml(ctx.fs, path.dirname(filePath));

  if (!cargoTomlPath) {
    return await ctx.crash({
      exitCode: 1,
      errorType: "invalid filesystem data",
      printedMessage:
        `No Cargo.toml found for ${filePath}.\n` +
        `Rust files in Convex must be part of a Cargo workspace or package.\n` +
        `Please create a Cargo.toml file in your convex directory or a parent directory.`,
    });
  }

  const projectDir = path.dirname(cargoTomlPath);

  // Determine the target WASM output path
  // We use wasm32-wasip1 for WASI support (enables system calls)
  const target = "wasm32-wasip1";
  const profile = "release";

  // Run cargo build
  try {
    const args = [
      "build",
      "--target",
      target,
      "--profile",
      profile,
      "--message-format=json",
    ];

    logVerbose(chalkStderr.yellow(`Running: cargo ${args.join(" ")}`));

    const { stdout, stderr } = await execFileAsync("cargo", args, {
      cwd: projectDir,
      encoding: "utf-8",
      maxBuffer: 10 * 1024 * 1024, // 10MB buffer for build output
    });

    if (stderr) {
      logWarning(chalkStderr.yellow(`Cargo build warnings: ${stderr}`));
    }

    // Parse cargo output to find the generated WASM file
    const wasmPath = parseCargoOutput(stdout, projectDir, profile);

    if (!wasmPath || !ctx.fs.exists(wasmPath)) {
      return await ctx.crash({
        exitCode: 1,
        errorType: "invalid filesystem data",
        printedMessage: `Failed to find compiled WASM output for ${filePath}`,
      });
    }

    // Read the WASM binary
    const wasmBuffer = ctx.fs.readFile(wasmPath);
    const wasmBinary = wasmBuffer.toString("base64");

    // Extract function metadata from the Rust source
    const functionMetadata = await extractFunctionMetadata(ctx, filePath);

    logVerbose(chalkStderr.green(`Successfully built Rust module: ${filePath}`));

    return {
      path: filePath,
      wasmBinary,
      functionMetadata,
      environment: "rust",
    };
  } catch (error: any) {
    return await ctx.crash({
      exitCode: 1,
      errorType: "invalid filesystem data",
      printedMessage:
        `Failed to compile Rust module ${filePath}:\n` + error.message,
    });
  }
}

// Parse cargo build output to find the WASM artifact path
function parseCargoOutput(
  stdout: string,
  projectDir: string,
  profile: string,
): string | null {
  const lines = stdout.split("\n");

  for (const line of lines) {
    try {
      const message = JSON.parse(line);
      // Look for compiler-artifact messages with WASM filenames
      if (
        message.reason === "compiler-artifact" &&
        message.target &&
        message.target.kind &&
        (message.target.kind.includes("cdylib") ||
          message.target.kind.includes("bin"))
      ) {
        // Find the WASM file in the filenames
        for (const filename of message.filenames || []) {
          if (filename.endsWith(".wasm")) {
            return filename;
          }
        }
      }
    } catch {
      // Skip non-JSON lines
      continue;
    }
  }

  return null;
}

// Extract function metadata from Rust source code
async function extractFunctionMetadata(
  ctx: Context,
  filePath: string,
): Promise<RustFunctionMetadata[]> {
  const metadata: RustFunctionMetadata[] = [];
  const source = ctx.fs.readUtf8File(filePath);

  // Simple regex-based extraction of #[query], #[mutation], #[action] attributes
  // In a real implementation, this would use a proper Rust parser (like rust-analyzer or syn)

  // Pattern to match: #[query] or #[query(...)] followed by async fn name
  const queryPattern =
    /#\[query(?:\([^)]*\))?\]\s*(?:#\[.*\]\s*)*\s*(?:pub\s+)?(?:async\s+)?fn\s+(\w+)/g;
  // Pattern to match: #[mutation] or #[mutation(...)]
  const mutationPattern =
    /#\[mutation(?:\([^)]*\))?\]\s*(?:#\[.*\]\s*)*\s*(?:pub\s+)?(?:async\s+)?fn\s+(\w+)/g;
  // Pattern to match: #[action] or #[action(...)]
  const actionPattern =
    /#\[action(?:\([^)]*\))?\]\s*(?:#\[.*\]\s*)*\s*(?:pub\s+)?(?:async\s+)?fn\s+(\w+)/g;

  let match;

  while ((match = queryPattern.exec(source)) !== null) {
    const name = match[1];
    metadata.push({
      name,
      functionType: "query",
      exportName: name,
    });
  }

  while ((match = mutationPattern.exec(source)) !== null) {
    const name = match[1];
    metadata.push({
      name,
      functionType: "mutation",
      exportName: name,
    });
  }

  while ((match = actionPattern.exec(source)) !== null) {
    const name = match[1];
    metadata.push({
      name,
      functionType: "action",
      exportName: name,
    });
  }

  return metadata;
}

// Build all Rust entry points
export async function bundleRustModules(
  ctx: Context,
  entryPoints: string[],
): Promise<RustBuildResult[]> {
  const results: RustBuildResult[] = [];

  for (const entryPoint of entryPoints) {
    const result = await buildRustModule(ctx, entryPoint);
    results.push(result);
  }

  return results;
}
