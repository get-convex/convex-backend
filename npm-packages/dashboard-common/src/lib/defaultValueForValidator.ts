import { ValidatorJSON, Value } from "convex/values";

export const defaultValueForValidator = (
  validator: ValidatorJSON,
): Value | undefined => {
  switch (validator.type) {
    case "null":
      return null;
    case "string":
      return "";
    case "boolean":
      return false;
    case "number":
      return 0;
    case "bigint":
      return BigInt(0);
    case "bytes":
      return new Uint8Array().buffer;
    case "any":
      return {};
    case "literal":
      return validator.value;
    case "id":
      return "";
    case "object":
      return Object.fromEntries(
        Object.entries(validator.value)
          .map(([fieldName, objectField]) => [
            fieldName,
            objectField.optional
              ? undefined
              : defaultValueForValidator(objectField.fieldType),
          ])
          // Remove undefined fields and undefined values
          .filter((d) => d !== undefined && d[1] !== undefined),
      );
    case "union":
      return defaultValueForValidator(validator.value[0]);
    case "record":
      return {};
    case "array":
      return [];
    default: {
      const _typeCheck: never = validator;
      throw new Error(
        `Unsupported validator type: ${JSON.stringify(validator)}`,
      );
    }
  }
};
