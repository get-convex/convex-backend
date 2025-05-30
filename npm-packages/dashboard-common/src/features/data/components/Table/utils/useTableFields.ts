import { useMemo } from "react";
import { Shape, topLevelFieldsFromShape } from "shapes";
import { GenericDocument } from "convex/server";
import {
  topLevelFieldsForValidator,
  useSingleTableEnforcedSchema,
} from "@common/features/data/components/TableSchema";
import { sortColumns } from "@common/features/data/lib/helpers";

export function useTableFields(
  tableName: string,
  shape: Shape | null,
  data?: GenericDocument[],
) {
  const tableSchema = useSingleTableEnforcedSchema(tableName);
  return useMemo(() => {
    const allFields = new Set<string>();
    if (tableSchema !== null) {
      const result = topLevelFieldsForValidator(
        tableSchema.documentType ?? { type: "any" },
      );
      // If schema is enforced and the list of fields is complete, use these.
      if (result.areFieldsComplete) {
        return sortColumns(result.fields);
      }
      result.fields.forEach((f) => allFields.add(f));
    }

    // TODO: Do we really need to look at the data to get all fields?
    // The schema + shape should be enough.
    data?.forEach((d) => Object.keys(d).forEach((f) => allFields.add(f)));

    // Add fields from shape
    const shapeFields = shape === null ? [] : topLevelFieldsFromShape(shape);
    shapeFields.forEach((f) => allFields.add(f));
    return sortColumns(Array.from(allFields));
  }, [tableSchema, data, shape]);
}
