import path from "path";
import { Context, logError } from "../../../../bundler/context.js";
import { DEFINITION_FILENAME, ROOT_DEFINITION_FILENAME } from "../constants.js";

/**
 * A component definition's location on the local filesystem,
 * using absolute paths.
 *
 * For module resolution it may be useful to avoid resolving any symlinks:
 * node modules are often symlinked by e.g. pnpm but relative paths should generally be
 * understood from their symlink location.
 *
 * None of these properties are the import string, which might have been an unqualifed import
 * (e.g. 'convex-waitlist' instead of '../node_modules/convex-waitlist/component.config.ts')
 */
export type ComponentDirectory = {
  isRoot: boolean;
  path: string;
  definitionPath: string;
};

/**
 * Qualify (ensure a leading dot) a path and make it relative to a working dir.
 * Qualifying a path clarifies to esbuild that it represents a local file system
 * path, not a remote path on the npm registry.
 *
 * Because the path is made relative without resolving symlinks this is a reasonable
 * identifier for the component directory (given a consistent working directory).
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

  // Check that we have a definition file.
  const filename = isRoot ? ROOT_DEFINITION_FILENAME : DEFINITION_FILENAME;
  const definitionPath = path.resolve(path.join(directory, filename));
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

/**
 * ComponentPath is the type of path sent to the server to identify a
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
  return path.relative(
    rootComponent.path,
    component.path,
  ) as ComponentDefinitionPath;
}

export function toAbsolutePath(
  rootComponent: ComponentDirectory,
  componentDefinitionPath: ComponentDefinitionPath,
) {
  return path.normalize(path.join(rootComponent.path, componentDefinitionPath));
}
