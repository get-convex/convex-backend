import { Infer, v } from "convex/values";

export const courseValidator = v.union(
  v.literal("appetizer"),
  v.literal("main"),
  v.literal("dessert"),
);

// The corresponding type can be used in server or client-side helpers:
export type Course = Infer<typeof courseValidator>;
// is inferred as `'appetizer' | 'main' | 'dessert'`
