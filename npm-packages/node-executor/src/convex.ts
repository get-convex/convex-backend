// Same structure as in the convex npm package.
export interface ConvexFunction {
  isAction: true;
  invokeAction(requestId: string, argsStr: string): Promise<string>;
}

// Matches the structure defined in Convex.
export function isConvexAction(object: any): object is ConvexFunction {
  return (
    (object.isAction ?? false) === true &&
    typeof object.invokeAction === "function"
  );
}

export type CanonicalizedModulePath = string;

// A module path alongside a named exported function within that module to execute.
export interface UdfPath {
  canonicalizedPath: CanonicalizedModulePath;
  function?: string;
}

// A module along its source.
export interface ModuleConfig {
  /// Relative path to the module.
  canonicalizedPath: CanonicalizedModulePath;
  /// Module source.
  source: string;
  /// The module's source map (if available).
  sourceMap?: string;
}

export type FunctionName = string;

export type Identifier = string;
export type LineNumber = number; // 1-indexed

export type EnvironmentVariable = {
  name: string;
  value: string;
};
