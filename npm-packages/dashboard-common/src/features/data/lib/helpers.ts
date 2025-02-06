import { useQuery } from "convex/react";
import { Value } from "convex/values";
import { useMemo } from "react";
import { Shape } from "shapes";
import udfs from "@common/udfs";
import { useNents } from "@common/lib/useNents";
import { SchemaJson } from "@common/lib/format";

export function sortColumns(fieldNames: string[]): string[] {
  // Always sort the "_id" field first and the "_creationTime" field last.
  return fieldNames.sort((a, b) => {
    if (a === b) {
      return 0;
    }
    if (a === "_id" || b === "_creationTime") {
      return -1;
    }
    if (b === "_id" || a === "_creationTime") {
      return 1;
    }
    if (a < b) {
      return -1;
    }
    return 1;
  });
}

export const validateConvexIdentifier = (identifier: string, name: string) =>
  identifier === ""
    ? `${name} cannot be empty.`
    : identifier.startsWith("_")
      ? `${name} cannot start with an underscore.`
      : identifier.match(/^[a-zA-Z_][\w]*$/) === null
        ? `${name} must only contain alphanumeric characters or underscores and can't start with a number.`
        : identifier.length > 64
          ? `${name} must be 64 characters or less.`
          : undefined;

export const identifierNeedsEscape = (identifier: string) =>
  identifier !== "_id" &&
  identifier !== "_creationTime" &&
  validateConvexIdentifier(identifier, "Field name") !== undefined;

export const defaultValueForShape = (shape: Shape): Value | undefined => {
  switch (shape.type) {
    case "Id":
      return "";
    case "String":
      return "";
    case "Boolean":
      return false;
    case "Float64":
      return 0;
    case "Int64":
      return BigInt(0);
    case "Array":
      return [];
    case "Object":
      return Object.fromEntries(
        shape.fields
          .map(({ fieldName, shape: fieldShape }) => [
            fieldName,
            defaultValueForShape(fieldShape),
          ])
          .filter((d) => d !== undefined),
      );
    case "Union":
      return defaultValueForShape(shape.shapes[0]);
    case "Record":
      return {};
    case "Null":
      return null;
    case "Bytes":
    case "Map":
    case "Never":
    case "Set":
    case "Unknown":
      return undefined;
    default: {
      const _typeCheck: never = shape;
      return undefined;
    }
  }
};

const COMMON_UTC_TIMESTAMP_RANGE = [1e12, 2e12]; // ~2001 to 2033
export const isInCommonUTCTimestampRange = (value: number) =>
  value > COMMON_UTC_TIMESTAMP_RANGE[0] &&
  value < COMMON_UTC_TIMESTAMP_RANGE[1];

export const useActiveSchema = () => {
  const schemas = useQuery(udfs.getSchemas.default, {
    componentId: useNents().selectedNent?.id ?? null,
  });
  const schema = useMemo(() => {
    if (schemas === undefined) {
      return undefined;
    }
    return schemas.active !== undefined
      ? (JSON.parse(schemas.active) as SchemaJson)
      : null;
  }, [schemas]);

  return schema;
};

export const isTableMissingFromSchema = (
  tableName: string,
  schema?: SchemaJson | null,
) => {
  if (!schema) {
    return false;
  }

  const tableNames = schema.tables.map((t) => t.tableName);
  return !tableNames.includes(tableName);
};
