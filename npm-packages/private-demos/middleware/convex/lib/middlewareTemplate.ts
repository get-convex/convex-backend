import { Auth } from "convex/server";
import { ObjectType, v } from "convex/values";
import { DatabaseReader, query } from "../_generated/server";
import {
  generateActionWithMiddleware,
  generateMiddlewareContextOnly,
  generateMutationWithMiddleware,
  generateQueryWithMiddleware,
} from "./middlewareUtils";

// ----------------------------------------------------------------------
// Fill these in:

// Things required in `ctx` for your middleware
type MyRequiredContext = { db: DatabaseReader; auth: Auth };

// Things added / replaced in `ctx` by your middleware -- functions
// using your middleware can access these
type TransformedContext = { user: string };

// Arguments consumed by your middleware -- functions using your middleware
// cannot access these unless you include them in `TransformedCtx`
const myValidator = { myArg: v.string() };

// The transformation your middleware is doing
const myTransform = async (
  ctx: MyRequiredContext,
  args: ObjectType<typeof myValidator>,
): Promise<TransformedContext> => {
  // Change this
  return { user: args.myArg };
};
// ----------------------------------------------------------------------

// ----------------------------------------------------------------------
// No need to modify these aside from renaming

// Helper function to allow applying this transform to multiple types of `Context`
// (e.g. QueryCtx, MutaitonCtx)
const myTransformGeneric = async <Ctx extends Record<string, any>>(
  ctx: Ctx & MyRequiredContext,
  args: ObjectType<typeof myValidator>,
): Promise<Omit<Ctx, keyof TransformedContext> & TransformedContext> => {
  return { ...ctx, ...(await myTransform(ctx, args)) };
};

export const withMyTransform = generateMiddlewareContextOnly(
  myValidator,
  myTransform,
);

export const queryWithMyTransform = generateQueryWithMiddleware(
  myValidator,
  myTransformGeneric,
);

export const mutationWithMyTransform = generateMutationWithMiddleware(
  myValidator,
  myTransformGeneric,
);

export const actionWithMyTransform = generateActionWithMiddleware(
  myValidator,
  // @ts-expect-error -- This isn't allowed since `MyRequiredCtx` requires
  // db, which is not available in actions
  myTransformGeneric,
);

// ----------------------------------------------------------------------
// Examples

const _q1 = query(
  withMyTransform({
    args: { a: v.string() },
    handler: async ({ db, user }, { a }) => {
      // Do stuff!
      console.log(user, db, a);
    },
  }),
);

const _q2 = queryWithMyTransform({
  args: { a: v.string() },
  handler: async ({ db, user }, { a }) => {
    // Do stuff!
    console.log(user, db, a);
  },
});
