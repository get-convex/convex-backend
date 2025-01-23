import { ValidatorJSON } from "convex/values";
import { SchemaJson } from "dashboard-common";

export function documentValidatorForTable(
  activeSchema: SchemaJson,
  tableName: string,
) {
  const documentValidator = activeSchema.tables.find(
    (t) => t.tableName === tableName,
  )?.documentType;
  return documentValidator;
}

export function validatorForColumn(
  documentValidator: SchemaJson["tables"][0]["documentType"],
  columnName: string,
): ValidatorJSON | undefined {
  const validator =
    documentValidator?.type === "object"
      ? documentValidator.value[columnName]?.fieldType
      : undefined;
  return validator;
}
