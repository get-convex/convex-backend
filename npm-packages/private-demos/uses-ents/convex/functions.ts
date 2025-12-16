import { entsTableFactory } from "convex-ents";
import {
  mutation as baseMutation,
  MutationCtx,
  query as baseQuery,
  QueryCtx,
} from "./_generated/server";
import { entDefinitions } from "./schema";

export const query = baseQuery;
export const mutation = baseMutation;

export function getQueryTable(ctx: QueryCtx) {
  return entsTableFactory(ctx, entDefinitions);
}

export function getMutationTable(ctx: MutationCtx) {
  return entsTableFactory(ctx, entDefinitions);
}
