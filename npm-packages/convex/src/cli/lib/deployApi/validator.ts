import { z } from "zod";

const baseConvexValidator = z.discriminatedUnion("type", [
  z.object({ type: z.literal("null") }),
  z.object({ type: z.literal("number") }),
  z.object({ type: z.literal("bigint") }),
  z.object({ type: z.literal("boolean") }),
  z.object({ type: z.literal("string") }),
  z.object({ type: z.literal("bytes") }),
  z.object({ type: z.literal("any") }),
  z.object({ type: z.literal("literal"), value: z.any() }),
  z.object({ type: z.literal("id"), tableName: z.string() }),
]);
export type ConvexValidator =
  | z.infer<typeof baseConvexValidator>
  | { type: "array"; value: ConvexValidator }
  | { type: "record"; keys: ConvexValidator; values: ConvexValidator }
  | { type: "union"; value: ConvexValidator[] }
  | {
      type: "object";
      value: Record<string, { fieldType: ConvexValidator; optional: boolean }>;
    };
export const convexValidator: z.ZodType<ConvexValidator> = z.lazy(() =>
  z.union([
    baseConvexValidator,
    z.object({ type: z.literal("array"), value: convexValidator }),
    z.object({
      type: z.literal("record"),
      keys: convexValidator,
      values: convexValidator,
    }),
    z.object({ type: z.literal("union"), value: z.array(convexValidator) }),
    z.object({
      type: z.literal("object"),
      value: z.record(
        z.object({ fieldType: convexValidator, optional: z.boolean() }),
      ),
    }),
  ]),
);
