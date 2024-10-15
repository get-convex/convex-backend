import { header } from "./common.js";

export function importPath(modulePath: string) {
  // Replace backslashes with forward slashes.
  const filePath = modulePath.replace(/\\/g, "/");
  // Strip off the file extension.
  const lastDot = filePath.lastIndexOf(".");
  return filePath.slice(0, lastDot === -1 ? undefined : lastDot);
}

export function moduleIdentifier(modulePath: string) {
  // TODO: This encoding is ambiguous (`foo/bar` vs `foo_bar` vs `foo-bar`).
  // Also we should be renaming keywords like `delete`.
  let safeModulePath = importPath(modulePath)
    .replace(/\//g, "_")
    .replace(/-/g, "_");
  // Escape existing variable names in this file
  if (["fullApi", "api", "internal", "components"].includes(safeModulePath)) {
    safeModulePath = `${safeModulePath}_`;
  }
  return safeModulePath;
}

export function apiCodegen(modulePaths: string[]) {
  const apiDTS = `${header("Generated `api` utility.")}
  import type { ApiFromModules, FilterApi, FunctionReference } from "convex/server";
  ${modulePaths
    .map(
      (modulePath) =>
        `import type * as ${moduleIdentifier(modulePath)} from "../${importPath(
          modulePath,
        )}.js";`,
    )
    .join("\n")}

  /**
   * A utility for referencing Convex functions in your app's API.
   *
   * Usage:
   * \`\`\`js
   * const myFunctionReference = api.myModule.myFunction;
   * \`\`\`
   */
  declare const fullApi: ApiFromModules<{
    ${modulePaths
      .map(
        (modulePath) =>
          `"${importPath(modulePath)}": typeof ${moduleIdentifier(modulePath)},`,
      )
      .join("\n")}
  }>;
  export declare const api: FilterApi<typeof fullApi, FunctionReference<any, "public">>;
  export declare const internal: FilterApi<typeof fullApi, FunctionReference<any, "internal">>;
  `;

  const apiJS = `${header("Generated `api` utility.")}
  import { anyApi } from "convex/server";

  /**
   * A utility for referencing Convex functions in your app's API.
   *
   * Usage:
   * \`\`\`js
   * const myFunctionReference = api.myModule.myFunction;
   * \`\`\`
   */
  export const api = anyApi;
  export const internal = anyApi;
  `;
  return {
    DTS: apiDTS,
    JS: apiJS,
  };
}
