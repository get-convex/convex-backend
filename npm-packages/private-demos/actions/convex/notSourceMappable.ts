import { action } from "./_generated/server";

export const nop = action({
  args: {},
  handler: async () => {},
});

/**
 * This function is not source-mappable by our current analyze because this file
 * include a helper function used in another entry point. That makes the bundler
 * stick both of these functions in deps/ file and makes this file just a
 * re-export of that nop function.
 */

export function helper(a: number, b: number): number {
  return a + b;
}
