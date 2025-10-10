import { useMemo } from "react";
import { Shape, topLevelFieldsFromShape } from "shapes";
import { topLevelFieldsForValidator } from "@common/features/data/components/TableSchema";
import { sortColumns } from "@common/features/data/lib/helpers";
import { SchemaJson } from "@common/lib/format";

export function useTableFields(
  tableName: string,
  shape: Shape | null,
  activeSchema: SchemaJson | null,
) {
  return useMemo(() => {
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
}
