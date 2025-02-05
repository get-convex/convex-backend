import { ValidatorJSON, Value } from "convex/values";
import isPlainObject from "lodash/isPlainObject";
import { UNDEFINED_PLACEHOLDER } from "system-udfs/convex/_system/frontend/patchDocumentsFields";
import {
  ConvexSchemaValidationError,
  ConvexValidationError,
  isUndefined,
  Node,
  ObjectPropertyNode,
} from "@common/elements/ObjectEditor/ast/types";
import { Walker } from "@common/elements/ObjectEditor/ast/ast";

// This function walks an AST when the mode is "patchDocuments".
export function walkPatchDocuments(node: Node, validator?: ValidatorJSON) {
  // Since we're patching multiple documents, we should make sure the value is an object,
  // and that any undefined values in the object are removed from the nodes to walk,
  // but still validated.
  const extractResult = extractUndefinedValuesFromNode(node, validator);
  const { node: nodeToWalk, errors: errorsToMerge } = extractResult;
  let { value } = extractResult;

  // If the validator type is a union, we should ignore it.
  // We don't have a better way to handle validating patches for unions yet.
  const walker = new Walker({
    validator: validator?.type === "union" ? undefined : validator,
  });
  const { value: walkerValue, errors: walkErrors } = walker.walk(
    nodeToWalk,
    false,
  );
  if (!isPlainObject(walkerValue)) {
    return {
      value: walkerValue,
      errors: [
        new ConvexValidationError("Value must be an object.", nodeToWalk.loc),
      ],
    };
  }

  // @ts-expect-error -- we know walkerValue is an object
  value = value === undefined ? walkerValue : { ...value, ...walkerValue };

  // If we're patching documents, we should ignore RequiredPropertyMissing errors on any fields that were not
  // specified.
  const filteredWalkErrors = walkErrors.filter(
    (error: any) =>
      !(error instanceof ConvexSchemaValidationError) ||
      error.code !== "RequiredPropertyMissing",
  );
  return {
    value: value === undefined ? walkerValue : value,
    errors: [...errorsToMerge, ...filteredWalkErrors],
  };
}

const propertyKey = (property: ObjectPropertyNode) =>
  property.key.type === "Identifier"
    ? property.key.name
    : property.key.type === "Literal"
      ? typeof property.key.value === "string"
        ? property.key.value
        : undefined
      : undefined;

function extractUndefinedValuesFromNode(
  nodeToWalk: Node,
  validator?: ValidatorJSON,
) {
  if (nodeToWalk.type !== "ObjectExpression") {
    return {
      node: nodeToWalk,
      value: undefined,
      errors: [
        new ConvexValidationError("Value must be an object.", nodeToWalk.loc),
      ],
    };
  }

  const v: Record<string, Value> = {};
  const errors: ConvexValidationError[] = [];
  nodeToWalk.properties.forEach((property) => {
    if (isUndefined(property.value)) {
      const key = propertyKey(property);
      if (typeof key !== "string") {
        errors.push(
          new ConvexValidationError(
            "Unsupported field name: key must be a string.",
            property.key.loc,
          ),
        );
        return;
      }
      v[key] = UNDEFINED_PLACEHOLDER;
      if (validator && validator.type === "object") {
        const fieldValidator = validator.value[key];
        if (fieldValidator && !fieldValidator.optional) {
          errors.push(
            new ConvexSchemaValidationError(
              "RequiredPropertyMissing",
              validator,
              key,
              property.value.loc,
            ),
          );
        }
      }
    }
  });

  return {
    node: {
      ...nodeToWalk,
      properties: nodeToWalk.properties.filter(
        (property) => !isUndefined(property.value),
      ),
    },
    value: v,
    errors,
  };
}
