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
          return {
            fields: sortColumns(result.fields, { maintainOrder: true }),
            isComplete: true,
          };
        }
        result.fields.forEach((f) => allFields.add(f));
      }
    }

    // Add fields from shape
    const shapeFields = shape === null ? [] : topLevelFieldsFromShape(shape);
    shapeFields.forEach((f) => allFields.add(f));

    return { fields: sortColumns(Array.from(allFields)), isComplete: false };
  }, [tableName, shape, activeSchema]);

  const observedFields = useMemo(() => {
    const allFields = new Set<string>();
    data.forEach((document) => {
      Object.keys(document).forEach((field) => allFields.add(field));
    });
    return Array.from(allFields).sort();
  }, [data]);

  return useMemo(
    () => {
      if (shapeAndSchemaFields.isComplete) {
        return shapeAndSchemaFields.fields;
      }

      // The shape is computed asynchronously and may lag behind recent writes. Include the
      // fields observed in the loaded documents so they are always visible.
      const allFields = new Set([
        ...shapeAndSchemaFields.fields,
        ...observedFields,
      ]);

      return sortColumns(Array.from(allFields));
    },
    // Depend on the contents of observedFields rather than its identity so
    // that a new page of documents with the same fields keeps the same
    // fields array (consumers memoize on it).
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [shapeAndSchemaFields, JSON.stringify(observedFields)],
  );
}
