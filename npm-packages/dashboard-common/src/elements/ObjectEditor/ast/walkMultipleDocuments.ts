import { ValidatorJSON, Value } from "convex/values";
import { Walker } from "@common/elements/ObjectEditor/ast/ast";
import {
  ArrayNode,
  ConvexValidationError,
} from "@common/elements/ObjectEditor/ast/types";

export function walkMultipleDocuments(
  array: ArrayNode,
  validator?: ValidatorJSON,
) {
  const startNodes = array.elements;
  const value: Value = [];
  const errors: ConvexValidationError[] = [];
  for (const node of startNodes) {
    if (node === null) {
      return {
        value: null,
        errors: [
          new ConvexValidationError(
            "Arrays must not have empty elements",
            array.loc,
          ),
        ],
      };
    }
    const walker = new Walker({ validator });
    const { value: walkerValue, errors: walkErrors } = walker.walk(node, true);
    value.push(walkerValue);
    errors.push(...walkErrors);
  }
  return { value, errors };
}
