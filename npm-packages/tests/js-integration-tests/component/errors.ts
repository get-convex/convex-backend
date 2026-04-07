import { query } from "./_generated/server";
import { ConvexError } from "convex/values";

export const throwConvexError = query(async () => {
  throw new ConvexError("Boom boom bop");
});

export const throwError = query(async () => {
  throw new Error("component kaboom");
});
