import path from "path";
import { Context } from "../../../../bundler/context.js";
import {
  DEFINITION_FILENAME_JS,
  DEFINITION_FILENAME_TS,
  DEFINITION_FILENAME_MJS,
} from "../constants.js";
import { getFunctionsDirectoryPath } from "../../config.js";

/**
 * A component definition's location on the local filesystem using absolute paths.
 *
 * For module resolution it would be useful to avoid resolving any symlinks:
 * node modules are often symlinked by e.g. pnpm but relative paths should generally be
 * understood from their symlink location. We don't currently do this though, it made
 * Windows harder to support.
 *
 * None of these properties are the import string, which might have been an unqualifed import
 * (e.g. 'convex-waitlist' instead of '../node_modules/convex-waitlist/convex.config.ts')
 */
export type ComponentDirectory = {
  /**
   * Is this component directory for the root component?
   */
  isRoot: boolean;

  /**
   * Absolute local filesystem path to the component definition's directory.
   */
  path: string;

  /**
   * Absolute local filesystem path to the `convex.config.{mjs,js,ts}` file within the component definition.
   */
  definitionPath: string;
};

/**
 * Qualify (ensure a leading dot) a path and make it relative to a working dir.
 * Qualifying a path clarifies to esbuild that it represents a local file system
 * path, not a remote path on the npm registry.
 *
 * If this path were made relative without resolving symlinks it would be a
 * prettier identifier for the component directory, but instead symlinks are
 * always resolved.
 */
export function qualifiedDefinitionPath(
  directory: ComponentDirectory,
  workingDir = ".",
) {
  const definitionPath = path.relative(workingDir, directory.definitionPath);
  return `./${definitionPath}`;
}

// NB: The process cwd will be used to resolve the directory specified in the constructor.
export function isComponentDirectory(
  ctx: Context,
  directory: string,
  isRoot: boolean,
):
  | { kind: "ok"; component: ComponentDirectory }
  | { kind: "err"; why: string } {
  if (!ctx.fs.exists(directory)) {
    return { kind: "err", why: `Directory doesn't exist` };
  }
  const dirStat = ctx.fs.stat(directory);
  if (!dirStat.isDirectory()) {
    return { kind: "err", why: `Not a directory` };
  }

  // Check that we have a definition file, using priority order: .mjs, .js, .ts
  const candidates = [
    DEFINITION_FILENAME_MJS,
    DEFINITION_FILENAME_JS,
    DEFINITION_FILENAME_TS,
  ];
  
  let filename = "";
  let definitionPath = "";
  
  for (const candidate of candidates) {
    const candidatePath = path.resolve(path.join(directory, candidate));
    if (ctx.fs.exists(candidatePath)) {
      filename = candidate;
      definitionPath = candidatePath;
      break;
    }
  }
  
  if (!filename) {
    return {
      kind: "err",
      why: `Directory doesn't contain any of the supported definition files: ${candidates.join(", ")}`,
    };
  }
  const definitionStat = ctx.fs.stat(definitionPath);
  if (!definitionStat.isFile()) {
    return {
      kind: "err",
      why: `Component definition ${filename} isn't a file`,
    };
  }
  return {
    kind: "ok",
    component: {
      isRoot,
      path: path.resolve(directory),
      definitionPath: definitionPath,
    },
  };
}

export async function buildComponentDirectory(
  ctx: Context,
  definitionPath: string,
): Promise<ComponentDirectory> {
  const convexDir = path.resolve(await getFunctionsDirectoryPath(ctx));
  const isRoot = path.dirname(path.resolve(definitionPath)) === convexDir;
  const isComponent = isComponentDirectory(
    ctx,
    path.dirname(definitionPath),
    isRoot,
  );
  if (isComponent.kind === "err") {
    return await ctx.crash({
      exitCode: 1,
      errorType: "invalid filesystem data",
      printedMessage: `Invalid component directory (${isComponent.why}): ${path.dirname(definitionPath)}`,
    });
  }
  return isComponent.component;
}

/**
 * ComponentPath is the local path identifying a
 * component definition. It is the unqualified (it never starts with "./")
 * relative path from the convex directory of the app (root component)
 * to the directory where a component definition lives.
 *
 * Note the convex/ directory of the root component is not necessarily
 * the working directory. It is currently never the same as the working
 * directory since `npx convex` must be invoked from the package root instead.
 */
export type ComponentDefinitionPath = string & {
  __brand: "ComponentDefinitionPath";
};

export function toComponentDefinitionPath(
  rootComponent: ComponentDirectory,
  component: ComponentDirectory,
): ComponentDefinitionPath {
  // First, compute a file system relative path.
  const relativePath: string = path.relative(
    rootComponent.path,
    component.path,
  );

  // Then, convert it to a ComponentDefinitionPath, which always uses POSIX conventions.
  const definitionPath = relativePath.split(path.sep).join(path.posix.sep);

  return definitionPath as ComponentDefinitionPath;
}

export function toAbsolutePath(
  rootComponent: ComponentDirectory,
  componentDefinitionPath: ComponentDefinitionPath,
) {
  // Repeat the process from `toComponentDefinitionPath` in reverse: First
  // convert to a relative local filesystem path, and then join it to
  // the root component's absolute path.
  const relativePath = componentDefinitionPath
    .split(path.posix.sep)
    .join(path.sep);
  return path.normalize(path.join(rootComponent.path, relativePath));
}
