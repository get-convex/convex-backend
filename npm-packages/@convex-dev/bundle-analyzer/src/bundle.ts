import {
  type BigBrainAuth,
  type BuildDebugInfo,
  type Context,
  type ErrorType,
  type Filesystem,
  bundleForAnalyzer,
  convexVersion,
  nodeFs,
  showSpinner,
  stopSpinner,
} from "convex/bundle-analyzer-internal";
import { BundleResult } from "./bundleResult.js";
import { AnalyzerError } from "./output.js";

/** The version of the convex source this analyzer uses. */
// Note that this is not entirely correct as the analyzer could potentially contain unreleased code.
export const analyzerConvexVersion: string = convexVersion;

/** Thrown by {@link ThrowingContext.crash}. */
class BundlerCrash extends Error {
  constructor(
    public readonly exitCode: number,
    public readonly errorType: ErrorType | undefined,
    public readonly printedMessage: string | null,
  ) {
    super(printedMessage ?? "Unknown error");
    this.name = "BundlerCrash";
  }
}

/**
 * A {@link Context} whose `crash` runs registered cleanups and then throws a
 * {@link BundlerCrash} instead of exiting the process.
 */
class ThrowingContext implements Context {
  fs: Filesystem = nodeFs;
  deprecationMessagePrinted = false;
  private cleanupFns: Record<
    string,
    (exitCode: number, err?: any) => Promise<void>
  > = {};
  private _bigBrainAuth: BigBrainAuth | null = null;

  async crash(args: {
    exitCode: number;
    errorType?: ErrorType;
    errForSentry?: any;
    printedMessage: string | null;
  }): Promise<never> {
    const cleanupFns = this.cleanupFns;
    this.cleanupFns = {};
    for (const fn of Object.values(cleanupFns)) {
      await fn(args.exitCode, args.errForSentry);
    }
    throw new BundlerCrash(args.exitCode, args.errorType, args.printedMessage);
  }

  registerCleanup(fn: (exitCode: number, err?: any) => Promise<void>): string {
    const handle = crypto.randomUUID();
    this.cleanupFns[handle] = fn;
    return handle;
  }

  removeCleanup(handle: string) {
    const value = this.cleanupFns[handle];
    delete this.cleanupFns[handle];
    return value ?? null;
  }

  bigBrainAuth(): BigBrainAuth | null {
    return this._bigBrainAuth;
  }

  _updateBigBrainAuth(auth: BigBrainAuth | null): void {
    this._bigBrainAuth = auth;
  }
}

/**
 * Bundles the project in the current directory with the convex bundler
 * embedded in this package, the way `npx convex dev` and `npx convex deploy`
 * do, without codegen or a deployment.
 */
export async function bundle(): Promise<BundleResult> {
  const ctx = new ThrowingContext();
  try {
    showSpinner("Bundling...");
    const { projectConfig, absWorkingDir, builds } = await bundleForAnalyzer(
      ctx,
      {
        cmd: "analyze bundles",
      },
    );
    return {
      projectRoot: absWorkingDir,
      includeSourcesContent:
        projectConfig.bundler?.includeSourcesContent ?? false,
      builds: builds.flatMap((b) => {
        // A build with no entry points (e.g. no `"use node"` modules) uploads
        // nothing. Its metafile is null or empty.
        if (b.metafile === null || b.modules.length === 0) return [];
        return [
          {
            component: {
              definitionPath: b.definitionPath,
              label: componentLabel(b),
              isRoot: b.directory.isRoot,
              directory: b.directory.path,
            },
            kind: b.kind,
            metafile: b.metafile,
            modules: b.modules.map((m) => ({
              path: m.path,
              source: m.source,
              sourceMap: m.sourceMap,
            })),
            externalDependencies: Object.fromEntries(b.externalDependencies),
          },
        ];
      }),
    };
  } catch (e) {
    if (e instanceof BundlerCrash) {
      throw new AnalyzerError(e.printedMessage ?? "Bundling failed.");
    }
    throw e;
  } finally {
    stopSpinner();
  }
}

/**
 * A human-readable name for the component a build belongs to: `app` for the
 * root, the package name for components installed from npm, and otherwise the
 * component's directory relative to the root's `convex/` directory.
 */
function componentLabel(build: BuildDebugInfo): string {
  const { directory, definitionPath } = build;
  if (directory.isRoot) {
    return "app";
  }
  if (directory.importSpecifier && !directory.importSpecifier.startsWith(".")) {
    return directory.importSpecifier;
  }
  const npmMatch = definitionPath.match(/node_modules\/((?:@[^/]+\/)?[^/]+)/);
  if (npmMatch) {
    return npmMatch[1];
  }
  return definitionPath;
}
