import { SchemaJson } from "../../../../../../lib/format";
import {
  documentValidatorForTable,
  validatorForColumn,
} from "../../utils/validators";

export function useValidator(
  activeSchema: SchemaJson | null,
  tableName: string,
  columnName: string,
) {
  const documentValidator =
    activeSchema && documentValidatorForTable(activeSchema, tableName);
  const validator = documentValidator
    ? validatorForColumn(documentValidator, columnName)
    : undefined;

  const shouldSurfaceValidatorErrors = activeSchema?.schemaValidation;

  let allowTopLevelUndefined = true;
  // If we're doing validation, and the column is not optional, we don't want to allow top-level undefined.
  if (
    validator &&
    documentValidator?.type === "object" &&
    !documentValidator.value[columnName]?.optional
  ) {
    allowTopLevelUndefined = false;
  }

  return { validator, shouldSurfaceValidatorErrors, allowTopLevelUndefined };
}
