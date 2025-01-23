import { ObjectType, PropertyValidators } from "convex/values";
import {
  ArgsArray,
  UnvalidatedFunction,
  ValidatedFunction,
  RegisteredQuery,
  RegisteredMutation,
  RegisteredAction,
} from "convex/server";
import {
  ActionCtx,
  MutationCtx,
  QueryCtx,
  action,
  mutation,
  query,
} from "../_generated/server";

export type MergeArgs<
  Args extends ArgsArray,
  Other extends { [k: string]: any },
> = Args extends [] ? [Other] : [Args[0] & Other];

export type MergeArgsForRegistered<
  Args extends ArgsArray,
  Other extends { [k: string]: any },
> = MergeArgs<Args, Other>[0];

export function splitArgs<
  ConsumedArgsValidator extends PropertyValidators,
  Args extends Record<string, any>,
>(
  consumedArgsValidator: ConsumedArgsValidator,
  args: Args & ObjectType<ConsumedArgsValidator>,
): { rest: Args; consumed: ObjectType<ConsumedArgsValidator> } {
  const rest: Record<string, any> = {};
  const consumed: Record<string, any> = {};
  for (const arg in args) {
    if (arg in consumedArgsValidator) {
      consumed[arg] = args[arg];
    } else {
      rest[arg] = args[arg];
    }
  }

  return {
    rest,
    consumed,
  } as any;
}

export const generateMiddlewareContextOnly = <
  OriginalCtx extends Record<string, any>,
  TransformedCtx extends Record<string, any>,
  ConsumedArgsValidator extends PropertyValidators,
>(
  consumedArgsValidator: ConsumedArgsValidator,
  transformContext: (
    ctx: OriginalCtx,
    args: ObjectType<ConsumedArgsValidator>,
  ) => Promise<TransformedCtx>,
) => {
  function withFoo<
    ExistingArgsValidator extends PropertyValidators,
    Output,
    Ctx,
  >(
    fn: ValidatedFunction<
      Ctx & TransformedCtx,
      ExistingArgsValidator,
      Promise<Output>
    >,
  ): ValidatedFunction<
    Ctx & OriginalCtx,
    ConsumedArgsValidator & ExistingArgsValidator,
    Promise<Output>
  >;

  function withFoo<ExistingArgs extends ArgsArray, Output, Ctx>(
    fn: UnvalidatedFunction<
      Ctx & TransformedCtx,
      ExistingArgs,
      Promise<Output>
    >,
  ): UnvalidatedFunction<
    Ctx & OriginalCtx,
    MergeArgs<ExistingArgs, ObjectType<ConsumedArgsValidator>>,
    Promise<Output>
  >;
  function withFoo(fn: any): any {
    if (fn.args) {
      const handler = fn.handler;
      return {
        args: {
          ...fn.args,
          ...consumedArgsValidator,
        },
        handler: async (ctx: any, allArgs: any) => {
          const { rest, consumed } = splitArgs(consumedArgsValidator, allArgs);
          const transformedCtx = await transformContext(ctx, consumed);
          return await handler(transformedCtx, rest);
        },
      };
    }
    const handler = fn.handler ?? fn;
    return {
      handler: async (ctx: any, allArgs: any) => {
        const { rest, consumed } = splitArgs(consumedArgsValidator, allArgs);
        const transformedCtx = await transformContext(ctx, consumed);
        return await handler(transformedCtx, rest);
      },
    };
  }

  return withFoo;
};

export const generateQueryWithMiddleware = <
  TransformedCtx extends Record<string, any>,
  ConsumedArgsValidator extends PropertyValidators,
>(
  consumedArgsValidator: ConsumedArgsValidator,
  transformContext: (
    ctx: QueryCtx,
    args: ObjectType<ConsumedArgsValidator>,
  ) => Promise<TransformedCtx>,
) => {
  const withFoo = generateMiddlewareContextOnly(
    consumedArgsValidator,
    transformContext,
  );

  function queryWithFoo<
    ExistingArgsValidator extends PropertyValidators,
    Output,
  >(
    fn: ValidatedFunction<
      TransformedCtx,
      ExistingArgsValidator,
      Promise<Output>
    >,
  ): RegisteredQuery<
    "public",
    ObjectType<ExistingArgsValidator> & ObjectType<ConsumedArgsValidator>,
    Output
  >;

  function queryWithFoo<ExistingArgs extends ArgsArray, Output>(
    fn: UnvalidatedFunction<TransformedCtx, ExistingArgs, Promise<Output>>,
  ): RegisteredQuery<
    "public",
    MergeArgsForRegistered<ExistingArgs, ObjectType<ConsumedArgsValidator>>,
    Output
  >;
  function queryWithFoo(fn: any): any {
    query(withFoo(fn));
  }

  return queryWithFoo;
};

export const generateMutationWithMiddleware = <
  TransformedCtx extends Record<string, any>,
  ConsumedArgsValidator extends PropertyValidators,
>(
  consumedArgsValidator: ConsumedArgsValidator,
  transformContext: (
    ctx: MutationCtx,
    args: ObjectType<ConsumedArgsValidator>,
  ) => Promise<TransformedCtx>,
) => {
  const withFoo = generateMiddlewareContextOnly(
    consumedArgsValidator,
    transformContext,
  );

  function mutationWithFoo<
    ExistingArgsValidator extends PropertyValidators,
    Output,
  >(
    fn: ValidatedFunction<
      TransformedCtx,
      ExistingArgsValidator,
      Promise<Output>
    >,
  ): RegisteredMutation<
    "public",
    ObjectType<ExistingArgsValidator> & ObjectType<ConsumedArgsValidator>,
    Output
  >;

  function mutationWithFoo<ExistingArgs extends ArgsArray, Output>(
    fn: UnvalidatedFunction<TransformedCtx, ExistingArgs, Promise<Output>>,
  ): RegisteredMutation<
    "public",
    MergeArgsForRegistered<ExistingArgs, ObjectType<ConsumedArgsValidator>>,
    Output
  >;
  function mutationWithFoo(fn: any): any {
    mutation(withFoo(fn));
  }

  return mutationWithFoo;
};

export const generateActionWithMiddleware = <
  TransformedCtx extends Record<string, any>,
  ConsumedArgsValidator extends PropertyValidators,
>(
  consumedArgsValidator: ConsumedArgsValidator,
  transformContext: (
    ctx: ActionCtx,
    args: ObjectType<ConsumedArgsValidator>,
  ) => Promise<TransformedCtx>,
) => {
  const withFoo = generateMiddlewareContextOnly(
    consumedArgsValidator,
    transformContext,
  );

  function actionWithFoo<
    ExistingArgsValidator extends PropertyValidators,
    Output,
  >(
    fn: ValidatedFunction<
      TransformedCtx,
      ExistingArgsValidator,
      Promise<Output>
    >,
  ): RegisteredAction<
    "public",
    ObjectType<ExistingArgsValidator> & ObjectType<ConsumedArgsValidator>,
    Output
  >;

  function actionWithFoo<ExistingArgs extends ArgsArray, Output>(
    fn: UnvalidatedFunction<TransformedCtx, ExistingArgs, Promise<Output>>,
  ): RegisteredAction<
    "public",
    MergeArgsForRegistered<ExistingArgs, ObjectType<ConsumedArgsValidator>>,
    Output
  >;
  function actionWithFoo(fn: any): any {
    action(withFoo(fn));
  }

  return actionWithFoo;
};
