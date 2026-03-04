import { useMemo } from "react";
import { GenericDocument } from "convex/server";
import { Shape, topLevelFieldsFromShape } from "shapes";
import { topLevelFieldsForValidator } from "@common/features/data/components/TableSchema";
import { sortColumns } from "@common/features/data/lib/helpers";
import { SchemaJson } from "@common/lib/format";

export function useTableFields(
  tableName: string,
  shape: Shape | null,
  activeSchema: SchemaJson | null,
  data: GenericDocument[],
) {
  const shapeAndSchemaFields = useMemo(() => {
    const allFields = new Set<string>();

    // Extract fields from the active schema
    if (activeSchema) {
      const tableSchema = activeSchema.tables.find(
        (table) => table.tableName === tableName,
      );
      if (tableSchema) {
        const result = topLevelFieldsForValidator(
          tableSchema.documentType ?? { type: "any" },
        );
        // If schema validation is enforced and fields are complete, use only these fields
        if (activeSchema.schemaValidation && result.areFieldsComplete) {
          return sortColumns(result.fields);
        }
        result.fields.forEach((f) => allFields.add(f));
      }
    }

    // Add fields from shape
    const shapeFields = shape === null ? [] : topLevelFieldsFromShape(shape);
    shapeFields.forEach((f) => allFields.add(f));

    return sortColumns(Array.from(allFields));
  }, [tableName, shape, activeSchema]);

  return useMemo(() => {
    // If we have data from schema or shapes, use it
    if (shapeAndSchemaFields.length > 0) {
      return shapeAndSchemaFields;
    }

    // No shape available — compute from the data itself.
    // This can happen when there is no schema and the computed shape
    // is a Record<string, …>, because there are two many fields or
    // some field names are invalid identifiers
    const allFields = new Set<string>();
    data.forEach((document) => {
      Object.keys(document).forEach((field) => allFields.add(field));
    });

    return sortColumns(Array.from(allFields));
  }, [shapeAndSchemaFields, data]);
}
