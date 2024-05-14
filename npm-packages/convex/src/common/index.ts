import type { Value } from "../values/value.js";

/**
 * Validate that the arguments to a Convex function are an object, defaulting
 * `undefined` to `{}`.
 */
export function parseArgs(
  args: Record<string, Value> | undefined,
): Record<string, Value> {
  if (args === undefined) {
    return {};
  }
  if (!isSimpleObject(args)) {
    throw new Error(
      `The arguments to a Convex function must be an object. Received: ${
        args as any
      }`,
    );
  }
  return args;
}

export function validateDeploymentUrl(deploymentUrl: string) {
  // Don't use things like `new URL(deploymentUrl).hostname` since these aren't
  // supported by React Native's JS environment
  if (typeof deploymentUrl === "undefined") {
    throw new Error(
      `Client created with undefined deployment address. If you used an environment variable, check that it's set.`,
    );
  }
  if (typeof deploymentUrl !== "string") {
    throw new Error(
      `Invalid deployment address: found ${deploymentUrl as any}".`,
    );
  }
  if (
    !(deploymentUrl.startsWith("http:") || deploymentUrl.startsWith("https:"))
  ) {
    throw new Error(
      `Invalid deployment address: Must start with "https://" or "http://". Found "${deploymentUrl}".`,
    );
  }

  // Skip validation on localhost because it's for internal Convex development.
  if (
    deploymentUrl.indexOf("127.0.0.1") !== -1 ||
    deploymentUrl.indexOf("localhost") !== -1
  ) {
    return;
  }

  if (
    !deploymentUrl.endsWith(".convex.cloud") &&
    !deploymentUrl.includes("0.0.0.0")
  ) {
    throw new Error(
      `Invalid deployment address: Must end with ".convex.cloud". Found "${deploymentUrl}". If you believe this URL is correct, use the \`skipConvexDeploymentUrlCheck\` option to bypass this.`,
    );
  }
}

/**
 * Check whether a value is a plain old JavaScript object.
 */
export function isSimpleObject(value: unknown) {
  const isObject = typeof value === "object";
  const prototype = Object.getPrototypeOf(value);
  const isSimple =
    prototype === null ||
    prototype === Object.prototype ||
    // Objects generated from other contexts (e.g. across Node.js `vm` modules) will not satisfy the previous
    // conditions but are still simple objects.
    prototype?.constructor?.name === "Object";
  return isObject && isSimple;
}
