import { GenericValidator } from "convex/values";
// This is where the alternatives are defined
import {
  // eslint-disable-next-line no-restricted-imports
  query as baseQuery,
  // eslint-disable-next-line no-restricted-imports
  mutation as baseMutation,
  // eslint-disable-next-line no-restricted-imports
  action as baseAction,
  // eslint-disable-next-line no-restricted-imports
  internalQuery as baseInternalQuery,
  // eslint-disable-next-line no-restricted-imports
  internalMutation as baseInternalMutation,
  // eslint-disable-next-line no-restricted-imports
  internalAction as baseInternalAction,
} from "../../_generated/server";
import {
  // eslint-disable-next-line no-restricted-imports
  queryGeneric as baseQueryGeneric,
  // eslint-disable-next-line no-restricted-imports
  mutationGeneric as baseMutationGeneric,
  // eslint-disable-next-line no-restricted-imports
  actionGeneric as baseActionGeneric,
  // eslint-disable-next-line no-restricted-imports
  internalQueryGeneric as baseInternalQueryGeneric,
  // eslint-disable-next-line no-restricted-imports
  internalMutationGeneric as baseInternalMutationGeneric,
  // eslint-disable-next-line no-restricted-imports
  internalActionGeneric as baseInternalActionGeneric,
  currentSystemUdfInComponent,
} from "convex/server";

import { DefaultFunctionArgs } from "convex/server";

type FunctionDefinition = {
  args: Record<string, GenericValidator>;
  returns?: GenericValidator;
  handler: (ctx: any, args: DefaultFunctionArgs) => any;
};

export const queryGeneric = baseQueryGeneric;
const mutationGenericWithoutComponent = baseMutationGeneric;
export const actionGeneric = baseActionGeneric;
export const internalQueryGeneric = baseInternalQueryGeneric;
export const internalMutationGeneric = baseInternalMutationGeneric;
export const internalActionGeneric = baseInternalActionGeneric;

export const mutationGeneric = ((functionDefinition: FunctionDefinition) => {
  return mutationGenericWithoutComponent({
    args: functionDefinition.args,
    returns: functionDefinition.returns,
    handler: async (ctx: any, args: any) => {
      if (
        "componentId" in args &&
        args.componentId !== null &&
        args.componentId !== undefined
      ) {
        const ref = currentSystemUdfInComponent(args.componentId);
        return await ctx.runMutation(ref, { ...args, componentId: null });
      }
      return functionDefinition.handler(ctx, args);
    },
  });
}) as typeof baseMutationGeneric;

// Specific to this schema.
export const query = baseQuery;
export const mutation = baseMutation;
export const action = baseAction;
export const internalQuery = baseInternalQuery;
export const internalMutation = baseInternalMutation;
export const internalAction = baseInternalAction;
