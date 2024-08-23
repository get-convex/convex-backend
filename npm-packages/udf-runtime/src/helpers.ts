import { performOp } from "udf-syscall-ffi";

/**
 * Throw an uncatchable error for unimplemented portions of JS builtins.
 *
 * We want to avoid obscure error messages due to code (i.e. external node modules)
 * try / catch-ing errors around JS builtins.
 * @param operation
 * @param className
 */
export const throwNotImplementedMethodError = (
  operation: string,
  className: string,
  extraMessage?: string,
): never => {
  const baseMessage = `Not implemented: ${operation} for ${className}`;
  const useNodeSuggestion =
    "Consider calling an action defined in Node.js instead (https://docs.convex.dev/functions/actions).";
  const message = extraMessage
    ? `${baseMessage}: ${extraMessage}. ${useNodeSuggestion}`
    : `${baseMessage}. ${useNodeSuggestion}`;
  return throwUncatchableDeveloperError(message);
};

/**
 * Throw an uncatchable error for unimplemented JS builtins.
 *
 * We want to avoid obscure error messages due to code (i.e. external node modules)
 * try / catch-ing errors around JS builtins.
 * @param operation
 * @param className
 */
export const throwNotImplementedError = (
  operation: string,
  extraMessage?: string,
): never => {
  const baseMessage = `Not implemented ${operation}`;
  const useNodeSuggestion =
    "Consider calling an action defined in Node.js instead (https://docs.convex.dev/functions/actions).";
  const message = extraMessage
    ? `${baseMessage}: ${extraMessage}. ${useNodeSuggestion}`
    : `${baseMessage}. ${useNodeSuggestion}`;
  return throwUncatchableDeveloperError(message);
};

export const throwUncatchableDeveloperError = (message: string): never => {
  // Make an error object so we can grab its stack trace and pass it through to
  // the syscall
  const error = new Error();
  // This calls `prepareStackTrace` that populates `__frameData`
  error.stack;
  const frameData = JSON.parse((error as any).__frameData ?? []);
  performOp("throwUncatchableDeveloperError", message, frameData);
  // This is not actually reachable because the syscall above will throw
  return null as never;
};

export function requiredArguments(length, required, prefix) {
  if (length < required) {
    const errMsg = `${prefix ? prefix + ": " : ""}${required} argument${
      required === 1 ? "" : "s"
    } required, but only ${length} present.`;
    throw new TypeError(errMsg);
  }
}
