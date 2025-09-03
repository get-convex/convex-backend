import { defineTable } from "convex/server";
import { v } from "convex/values";

export const awsLambdaVersionsTable = defineTable({
  lambdaName: v.string(),
  lambdaVersion: v.string(),
  lambdaConfig: v.object({
    runtime: v.string(),
  }),
  typeConfig: v.object({
    type: v.string(),
  }),
});
