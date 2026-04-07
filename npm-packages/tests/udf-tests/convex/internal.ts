import {
  internalQuery,
  internalMutation,
  query,
  mutation,
} from "./_generated/server";

/**
 * Line numbers in this file matter!
 *
 * They are tested by `test_analyze_internal_function` so if you change this
 * file you'll need to update the test.
 */

export const myInternalQuery = internalQuery(() => {
  // intentional noop.
});
export const publicQuery = query(() => {
  // intentional noop.
});

export const myInternalMutation = internalMutation(() => {
  // intentional noop.
});
export const publicMutation = mutation(() => {
  // intentional noop.
});
