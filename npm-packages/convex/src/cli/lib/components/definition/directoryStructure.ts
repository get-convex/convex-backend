import path from "path";
import { Context, logError } from "../../../../bundler/context.js";
import { DEFINITION_FILENAME, ROOT_DEFINITION_FILENAME } from "../constants.js";

/**
 * Absolute paths to a component on the local filesystem.
 *
 * For module resolution it's useful to avoid resolving any symlinks:
 * node modules may have different locations on disk, but should be understood
 * to exist at the location
 *
 * ComponentDirectory *could* store the unqualifed import string used to find it.
 * (e.g. 'convex-waitlist' instead of '../node_modules/convex-waitlist/component.config.ts')
 * but it doesn't.
 */
export type ComponentDirectory = {
  name: string;
  path: string;
  definitionPath: string;
};
// If you want an abspath don't use these things!
// Goals:
// 1. no absolute paths should be sent to a Convex deployments
// 2. convex/app.config.js is not a hardcoded locations, since functionsDir changes
// when functionsDir changes, ideally

/**
 * Qualify (ensure a leading dot) a path and make it relative to a working dir.
 * Qualifying a path clarifies that it represents a local file system path, not
 * a remote path on the npm registry.
 *
 * Because the path is made relative without resolving symlinks this is a reasonable
 * identifier for the component directory (given a consistent working directory).
 */
export function qualifiedDefinitionPath(
  directory: ComponentDirectory,
  workingDir = ".",
) {
  const definitionPath = path.relative(workingDir, directory.definitionPath);
  // A ./ or ../ prefix make a path "qualified."
  if (definitionPath.startsWith("./") || definitionPath.startsWith("../")) {
    return definitionPath;
  } else {
    return `./${definitionPath}`;
  }
}

// The process cwd will be used to resolve a componentPath specified in the constructor.
export function isComponentDirectory(
  ctx: Context,
  componentPath: string,
  isRoot: boolean,
):
  | { kind: "ok"; component: ComponentDirectory }
  | { kind: "err"; why: string } {
  if (!ctx.fs.exists(componentPath)) {
    return { kind: "err", why: `Directory doesn't exist` };
  }
  const dirStat = ctx.fs.stat(componentPath);
  if (!dirStat.isDirectory()) {
    return { kind: "err", why: `Not a directory` };
  }

  // Check that we have a definition file.
  const filename = isRoot ? ROOT_DEFINITION_FILENAME : DEFINITION_FILENAME;
  const definitionPath = path.resolve(path.join(componentPath, filename));
  if (!ctx.fs.exists(definitionPath)) {
    return {
      kind: "err",
      why: `Directory doesn't contain a ${filename} file`,
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
      name: isRoot ? "App" : path.basename(componentPath),
      path: path.resolve(componentPath),
      definitionPath: definitionPath,
    },
  };
}

export async function buildComponentDirectory(
  ctx: Context,
  definitionPath: string,
): Promise<ComponentDirectory> {
  const isRoot = path.basename(definitionPath) === ROOT_DEFINITION_FILENAME;
  const isComponent = isComponentDirectory(
    ctx,
    path.dirname(definitionPath),
    isRoot,
  );
  if (isComponent.kind === "err") {
    logError(
      ctx,
      `Invalid component directory (${isComponent.why}): ${path.dirname(definitionPath)}`,
    );
    return await ctx.crash(1, "invalid filesystem data");
  }
  return isComponent.component;
}

// A component path is a useful concept on the server.
// Absolute paths should not reach the server because deploying
// with a repo checked out to a different location should not result
// in changes.
// Paths relative to the root of a project should not reach the server
// for similar but more theoritical reasons: a given convex functions
// directory should configure a deployment regardless of the package.json
// being used to do the deploy.
export type ComponentPath = string;

export function toComponentPath(
  rootComponent: ComponentDirectory,
  component: ComponentDirectory,
) {
  return path.relative(rootComponent.path, component.path);
}

export function toAbsolutePath(
  rootComponent: ComponentDirectory,
  componentPath: ComponentPath,
) {
  return path.normalize(path.join(rootComponent.path, componentPath));
}
