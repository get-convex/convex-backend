import { z } from "zod";

const float64RangeSchema = z.object({
  hasSpecialValues: z.boolean().optional(),
});
export type Float64Range = z.infer<typeof float64RangeSchema>;

export type Shape =
  | { type: "Unknown" }
  | { type: "Never" }
  | { type: "Id"; tableName: string }
  | { type: "Null" }
  | { type: "Int64" }
  | { type: "Float64"; float64Range: Float64Range }
  | { type: "Boolean" }
  | { type: "String" }
  | { type: "Bytes" }
  | {
      type: "Object";
      fields: Array<{ fieldName: string; optional: boolean; shape: Shape }>;
    }
  | { type: "Array"; shape: Shape }
  | { type: "Set"; shape: Shape }
  | { type: "Map"; keyShape: Shape; valueShape: Shape }
  | { type: "Union"; shapes: Array<Shape> }
  | {
      type: "Record";
      keyShape: Shape;
      valueShape: { optional: boolean; shape: Shape };
    };

export const shapeSchema: z.ZodType<Shape> = z.lazy(() =>
  z.discriminatedUnion("type", [
    z.object({ type: z.literal("Unknown") }),
    z.object({ type: z.literal("Never") }),
    z.object({
      type: z.literal("Id"),
      tableName: z.string(),
    }),
    z.object({ type: z.literal("Null") }),
    z.object({
      type: z.literal("Int64"),
    }),
    z.object({
      type: z.literal("Float64"),
      float64Range: float64RangeSchema,
    }),
    z.object({ type: z.literal("Boolean") }),
    z.object({ type: z.literal("String") }),
    z.object({ type: z.literal("Bytes") }),
    z.object({
      type: z.literal("Object"),
      fields: z.array(
        z.object({
          fieldName: z.string(),
          optional: z.boolean(),
          shape: shapeSchema,
        }),
      ),
    }),
    z.object({
      type: z.literal("Array"),
      shape: shapeSchema,
    }),
    z.object({
      type: z.literal("Set"),
      shape: shapeSchema,
    }),
    z.object({
      type: z.literal("Map"),
      keyShape: shapeSchema,
      valueShape: shapeSchema,
    }),
    z.object({
      type: z.literal("Union"),
      shapes: z.array(shapeSchema),
    }),
    z.object({
      type: z.literal("Record"),
      keyShape: shapeSchema,
      valueShape: z.object({
        optional: z.boolean(),
        shape: shapeSchema,
      }),
    }),
  ]),
);

export function stringifyShape(shape: Shape): string {
  const variant = shape;
  switch (variant.type) {
    case "Array":
      return `Array<${stringifyShape(variant.shape)}>`;
    case "Boolean":
      return "boolean";
    case "Bytes":
      return "ArrayBuffer";
    case "Float64":
      return "number";
    case "Id":
      return `Id<"${variant.tableName}">`;
    case "Int64":
      return "bigint";
    case "Map":
      return `Map<${stringifyShape(variant.keyShape)},${stringifyShape(
        variant.valueShape,
      )}>`;
    case "Never":
      return "never";
    case "Null":
      return "null";
    case "Object": {
      const fieldsStr = variant.fields
        .map(
          (p) =>
            `${p.fieldName}${p.optional ? "?" : ""}: ${stringifyShape(p.shape)}`,
        )
        .join(",");
      return "{" + fieldsStr + "}";
    }
    case "Record": {
      if (variant.valueShape.optional) {
        return `{ [key in ${stringifyShape(
          variant.keyShape,
        )}]?: ${stringifyShape(variant.valueShape.shape)} }`;
      } else {
        return `Record<${stringifyShape(variant.keyShape)}, ${stringifyShape(
          variant.valueShape.shape,
        )}>`;
      }
    }
    case "Set":
      return `Set<${stringifyShape(variant.shape)}>`;
    case "String":
      return "string";
    case "Union":
      return variant.shapes.map(stringifyShape).join("|");
    case "Unknown":
      return "unknown";
    default: {
      variant satisfies never;
      throw new Error(`Unrecognized variant in ${shape}`);
    }
  }
}

export function topLevelFieldsFromShape(shape: Shape): Array<string> {
  if (shape.type === "Object") {
    return shape.fields.map((f) => f.fieldName);
  }
  if (shape.type === "Union") {
    return Array.from(
      new Set(shape.shapes.flatMap((s: Shape) => topLevelFieldsFromShape(s))),
    );
  }
  return [];
}
