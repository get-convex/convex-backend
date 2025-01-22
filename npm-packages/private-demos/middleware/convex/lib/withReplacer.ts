import { ObjectType, PropertyValidators, v } from "convex/values";
import { Doc } from "../_generated/dataModel";
import {
  ArgsArray,
  UnvalidatedFunction,
  ValidatedFunction,
} from "convex/server";
import { MergeArgs, splitArgs } from "./middlewareUtils";

const replacerConsumedArgs = { toReplace: v.string() };
type ConsumedArgsValidator = typeof replacerConsumedArgs;
type BaseOutput = string | { [k: string]: string | number };
type TransformedOutput<O extends BaseOutput> = O extends string
  ? { oldValue: string; newValue: string }
  : {
      [k in keyof O]: O[k] extends string
        ? { oldValue: string; newValue: string }
        : number;
    };
type BaseCtx = { session: Doc<"sessions"> };

const transformOutput = (
  output: BaseOutput,
  args: ObjectType<ConsumedArgsValidator>,
  ctx: BaseCtx,
): TransformedOutput<BaseOutput> => {
  if (typeof output === "string") {
    const replaced = output.replaceAll(
      args.toReplace,
      ctx.session.replacer ?? "default",
    );
    return { oldValue: output, newValue: replaced };
  }
  const replacedObj: any = {};
  for (const k of Object.keys(output)) {
    const value = output[k];
    if (typeof value === "string") {
      const replaced = value.replaceAll(
        "edge",
        ctx.session.replacer ?? "default",
      );
      replacedObj[k] = { oldValue: value, newValue: replaced };
    } else {
      replacedObj[k] = output[k];
    }
  }
  return replacedObj;
};

export function withReplacer<
  ExistingArgsValidator extends PropertyValidators,
  Ctx extends BaseCtx,
  Output extends BaseOutput,
>(
  fn: ValidatedFunction<Ctx, ExistingArgsValidator, Promise<Output>>,
): ValidatedFunction<
  Ctx,
  ConsumedArgsValidator & ExistingArgsValidator,
  Promise<TransformedOutput<Output>>
>;

export function withReplacer<
  ExistingArgs extends ArgsArray,
  Ctx extends BaseCtx,
  Output extends BaseOutput,
>(
  fn: UnvalidatedFunction<Ctx, ExistingArgs, Promise<Output>>,
): UnvalidatedFunction<
  Ctx,
  MergeArgs<ExistingArgs, ObjectType<ConsumedArgsValidator>>,
  Promise<TransformedOutput<Output>>
>;
export function withReplacer(fn: any): any {
  if (fn.args) {
    const handler = fn.handler;
    return {
      args: {
        ...fn.args,
        ...replacerConsumedArgs,
      },
      handler: async (ctx: any, allArgs: any) => {
        const { rest, consumed } = splitArgs(replacerConsumedArgs, allArgs);
        return await transformOutput(await handler(ctx, rest), consumed, ctx);
      },
    };
  }
  const handler = fn.handler ?? fn;
  return {
    handler: async (ctx: any, allArgs: any) => {
      const { rest, consumed } = splitArgs(replacerConsumedArgs, allArgs);
      return await transformOutput(await handler(ctx, rest), consumed, ctx);
    },
  };
}
