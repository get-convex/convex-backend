// Argument-validated versions of wrappers for use in system UDFs necessary
// because system UDFs are not analyzed.

import { GenericValidator, convexToJson } from "convex/values";
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
} from "../_generated/server";
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
import { performOp } from "udf-syscall-ffi";

type FunctionDefinition = {
  args: Record<string, GenericValidator>;
  returns?: GenericValidator;
  handler: (ctx: any, args: DefaultFunctionArgs) => any;
};

type WrappedFunctionDefinition = {
  args: Record<string, GenericValidator>;
  returns?: GenericValidator;
  handler: (ctx: any, args: DefaultFunctionArgs) => any;
  exportArgs(): string;
  exportReturns(): string;
};

type Wrapper = (def: FunctionDefinition) => WrappedFunctionDefinition;

function withArgsValidated<T>(wrapper: T): T {
  return ((functionDefinition: FunctionDefinition) => {
    if (!("args" in functionDefinition)) {
      throw new Error("args validator required for system udf");
    }
    const wrap: Wrapper = wrapper as Wrapper;
    const func = wrap({
      args: functionDefinition.args,
      returns: functionDefinition.returns,
      handler: () => {},
    });
    const argsValidatorJson = func.exportArgs();
    const returnsValidatorJson = func.exportReturns();
    return wrap({
      args: functionDefinition.args,
      returns: functionDefinition.returns,
      handler: async (ctx: any, args: any) => {
        const validateArgsResult = await performOp(
          "validateArgs",
          argsValidatorJson,
          convexToJson(args),
        );
        if (!validateArgsResult.valid) {
          throw new Error(validateArgsResult.message);
        }
        const functionResult = await functionDefinition.handler(ctx, args);
        const validateReturnsResult = await performOp(
          "validateReturns",
          returnsValidatorJson,
          convexToJson(functionResult === undefined ? null : functionResult),
        );
        if (!validateReturnsResult.valid) {
          throw new Error(validateReturnsResult.message);
        }
        return functionResult;
      },
    });
  }) as T;
}

export const queryGeneric = withArgsValidated(baseQueryGeneric);
const mutationGenericWithoutComponent = withArgsValidated(baseMutationGeneric);
export const actionGeneric = withArgsValidated(baseActionGeneric);
export const internalQueryGeneric = withArgsValidated(baseInternalQueryGeneric);
export const internalMutationGeneric = withArgsValidated(
  baseInternalMutationGeneric,
);
export const internalActionGeneric = withArgsValidated(
  baseInternalActionGeneric,
);

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
export const query = withArgsValidated(baseQuery);
export const mutation = withArgsValidated(baseMutation);
export const action = withArgsValidated(baseAction);
export const internalQuery = withArgsValidated(baseInternalQuery);
export const internalMutation = withArgsValidated(baseInternalMutation);
export const internalAction = withArgsValidated(baseInternalAction);
