import { useQuery } from "convex/react";
import { GenericDocument } from "convex/server";
import { useMemo } from "react";
import { Shape } from "shapes";
import { parseAndFilterToSingleTable } from "system-udfs/convex/_system/frontend/lib/filters";
import udfs from "@common/udfs";
import { Value } from "convex/values";
import { defaultValueForShape } from "@common/features/data/lib/helpers";
import { useTableShapes } from "@common/lib/deploymentApi";
import { useNents } from "@common/lib/useNents";
import { defaultValueForValidator } from "@common/lib/defaultValueForValidator";

export const useDefaultDocument = (tableName: string): GenericDocument => {
  const { tables } = useTableShapes();
  const shape = tables?.get(tableName)!;
  validateDocumentShape(shape);

  const schemas = useQuery(udfs.getSchemas.default, {
    componentId: useNents().selectedNent?.id ?? null,
  });

  const activeSchema = schemas?.active;

  const tableDef = activeSchema
    ? parseAndFilterToSingleTable(tableName, activeSchema)?.tables[0]
    : undefined;
  const defaultValueForSchema =
    tableDef &&
    defaultValueForValidator(tableDef.documentType ?? { type: "any" });
  const shapeFields = useMemo(
    () =>
      shape?.type === "Object"
        ? shape?.fields?.filter((field) => !field.fieldName.startsWith("_")) ||
          []
        : [],
    [shape],
  );
  // Initialize the document with default values from the shape.
  return useMemo(
    () =>
      (defaultValueForSchema as GenericDocument) ||
      shapeFields.reduce((acc: { [key: string]: Value }, curr) => {
        const defaultValue = defaultValueForShape(curr.shape);
        if (defaultValue !== undefined) {
          acc[curr.fieldName] = defaultValue;
        }
        return acc;
      }, {}),
    [defaultValueForSchema, shapeFields],
  );
};
const validateDocumentShape = (shape: Shape | undefined) => {
  if (shape === null || shape === undefined) {
    return;
  }
  switch (shape.type) {
    case "Object":
    case "Unknown":
    case "Never":
    case "Record":
      // These are all valid top level types for documents in a table.
      break;
    case "Array":
    case "Boolean":
    case "Bytes":
    case "Float64":
    case "Int64":
    case "Id":
    case "Map":
    case "Null":
    case "Set":
    case "String":
    case "Union":
      // Note that Union is not a valid top level shape since our algorithm
      // merges all objects into one, or uses a supertype like Record or Unknown
      throw new Error(`Table has invalid top level shape: ${shape.type}`);
    default: {
      const _typeCheck: never = shape;
      throw new Error(`Table with unexpected type: ${(shape as any).type}`);
    }
  }
};
