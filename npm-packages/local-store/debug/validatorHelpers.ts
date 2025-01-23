import { JSONValue } from "convex/values";

import { ObjectFieldType } from "convex/values";

export function formatValidator(validator: ValidatorJSON, indent = 0): string {
  switch (validator.type) {
    case "null":
      return "null";
    case "number":
      return "number";
    case "bigint":
      return "bigint";
    case "boolean":
      return "boolean";
    case "string":
      return "string";
    case "bytes":
      return "Uint8Array";
    case "any":
      return "any";
    case "literal":
      return JSON.stringify(validator.value);
    case "id":
      return `Id<"${validator.tableName}">`;
    case "array":
      return `${formatValidator(validator.value)}[]`;
    case "record": {
      const keyType = formatRecordKeyValidator(validator.keys);
      const valueType = formatValidator(validator.values.fieldType);
      return `Record<${keyType}, ${valueType}>`;
    }
    case "object":
      return `{\n${Object.entries(validator.value)
        .map(
          ([key, type]) =>
            `${"\t".repeat(indent + 1)}${key}${
              type.optional ? "?" : ""
            }: ${formatValidator(type.fieldType, indent + 1)}`,
        )
        .join(",\n")}\n${"\t".repeat(indent)}}`;
    case "union":
      return validator.value.map(formatValidator).join(" | ");
  }
}

function formatRecordKeyValidator(validator: RecordKeyValidatorJSON): string {
  switch (validator.type) {
    case "string":
      return "string";
    case "id":
      return `Id<"${validator.tableName}">`;
    case "union":
      return validator.value.map(formatRecordKeyValidator).join(" | ");
  }
}

export type ValidatorJSON =
  | { type: "null" }
  | { type: "number" }
  | { type: "bigint" }
  | { type: "boolean" }
  | { type: "string" }
  | { type: "bytes" }
  | { type: "any" }
  | { type: "literal"; value: JSONValue }
  | { type: "id"; tableName: string }
  | { type: "array"; value: ValidatorJSON }
  | {
      type: "record";
      keys: RecordKeyValidatorJSON;
      values: RecordValueValidatorJSON;
    }
  | { type: "object"; value: Record<string, ObjectFieldType> }
  | { type: "union"; value: ValidatorJSON[] };

type RecordKeyValidatorJSON =
  | { type: "string" }
  | { type: "id"; tableName: string }
  | { type: "union"; value: RecordKeyValidatorJSON[] };

type RecordValueValidatorJSON = ObjectFieldType & { optional: false };
