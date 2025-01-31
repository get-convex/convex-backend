import { useQuery } from "convex/react";
import { useMemo } from "react";
import { Shape } from "shapes";
import udfs from "udfs";
import { parseAndFilterToSingleTable } from "system-udfs/convex/_system/frontend/lib/filters";
import { SchemaJson } from "lib/format";
import { useNents } from "lib/useNents";
import { useTableShapes } from "lib/deploymentApi";
import { LoadingTransition } from "elements/Loading";
import { ShowSchema } from "features/data/components/ShowSchema";

export interface TableSchemaStatus {
  tableName: string;
  isDefined: boolean;
  referencedByTable: string | undefined;
  isValidationRunning: boolean;
}

interface Schema {
  tables: Table[];
  schemaValidation: boolean;
}
export interface Table {
  tableName: string;
  documentType: Validator;
}
type Validator =
  | { type: "null" }
  | { type: "number" }
  | { type: "bigint" }
  | { type: "boolean" }
  | { type: "string" }
  | { type: "boolean" }
  | { type: "bytes" }
  | { type: "any" }
  | { type: "literal"; value: any }
  | { type: "id"; tableName: string }
  | { type: "array"; value: Validator }
  | { type: "record"; keys: Validator; values: Validator }
  | { type: "union"; value: Validator[] }
  | {
      type: "object";
      value: Record<string, { fieldType: Validator; optional: boolean }>;
    };

function validatorReferencesTable(
  validator: Validator,
  tableName: string,
): boolean {
  switch (validator.type) {
    case "id":
      return validator.tableName === tableName;
    case "array":
      return validatorReferencesTable(validator.value, tableName);
    case "record":
      return (
        validatorReferencesTable(validator.keys, tableName) ||
        validatorReferencesTable(validator.values, tableName)
      );
    case "union":
      return validator.value.some((v) =>
        validatorReferencesTable(v, tableName),
      );
    case "object":
      return Object.values(validator.value).some((v) =>
        validatorReferencesTable(v.fieldType, tableName),
      );
    default:
      return false;
  }
}

export function useSingleTableSchemaStatus(
  tableName: string,
): TableSchemaStatus | undefined {
  const schemas = useQuery(udfs.getSchemas.default, {
    componentId: useNents().selectedNent?.id ?? null,
  });
  if (!schemas) {
    return undefined;
  }
  const active: Schema | undefined = schemas.active
    ? JSON.parse(schemas.active)
    : undefined;
  const isDefined =
    active?.tables.find((table) => table.tableName === tableName) !== undefined;
  const referencedByTable = active?.tables.find((table) =>
    validatorReferencesTable(table.documentType, tableName),
  )?.tableName;
  const isValidationRunning = schemas.inProgress !== undefined;
  return {
    tableName,
    isDefined,
    referencedByTable,
    isValidationRunning,
  };
}

export function useSingleTableEnforcedSchema(tableName: string): Table | null {
  const schemas = useQuery(udfs.getSchemas.default, {
    componentId: useNents().selectedNent?.id ?? null,
  });
  if (!schemas) {
    return null;
  }
  const active: Schema | undefined = schemas.active
    ? JSON.parse(schemas.active)
    : undefined;
  if (active?.schemaValidation === false) {
    return null;
  }
  const tableSchema = active?.tables.find(
    (table) => table.tableName === tableName,
  );
  return tableSchema ?? null;
}

// Adds a comment '// Other tables here...' into code, assuming that code
// represents a valid schema.ts file containing only one table whose name is
// the given tableName.
function insertOtherTablesComment(tableName: string, code: string): string {
  const splitLines = code.split("\n");

  // Use a token to find the start of the table definition
  // assumes that the string 'tableName: defineTable' is unique in the file
  const tableNameToken = `${tableName}: defineTable`;
  const tableStartIndex = splitLines.findIndex((value) =>
    value.trim().includes(tableNameToken),
  );

  // Check for a single line table definition definition like:
  // export default defineSchema({ Test: defineTable({ my_id: s.string() }) });
  const tableStartLine = splitLines[tableStartIndex];
  const isSingleLineDefinition = tableStartLine
    .trim()
    .startsWith("export default defineSchema");
  if (isSingleLineDefinition) {
    const tableStartIndexInLine = tableStartLine.indexOf(tableNameToken);

    const chars = tableStartLine.split("");
    // -1 to remove the ' ' character
    chars.splice(tableStartIndexInLine - 1, 1, "\n  ");
    // export default defineSchema({
    //   Test: defineTable({ my_id: s.string() }) });
    // -1 to remove the ' ' character
    chars.splice(chars.length - 4, 1, "\n");
    // export default defineSchema({
    //   Test: defineTable({ my_id: s.string() })
    // });
    const linesToAdd = chars.join("").split("\n");
    splitLines.splice(tableStartIndex, 1, ...linesToAdd);
  }

  splitLines.splice(
    tableStartIndex + (isSingleLineDefinition ? 1 : 0),
    0,
    "  // Other tables here...\n",
  );
  // export default defineSchema({
  //   // Other tables here...
  //
  //   Test: defineTable({ my_id: s.string() })
  // });

  return splitLines.join("\n");
}

function TableSchema({
  tables,
  tableName,
  activeSchema,
  inProgressSchema,
  hadShapeError,
}: {
  tables: Map<string, Shape>;
  tableName: string;
  activeSchema?: SchemaJson;
  inProgressSchema?: SchemaJson;
  hadShapeError: boolean;
}) {
  const tableShape = tables.get(tableName);
  if ((!tableShape || tableShape.type === "Never") && !activeSchema) {
    return (
      <div className="sm:px-2">
        Add at least one document to your table to see a suggested schema here.
      </div>
    );
  }

  const singleTableShapes = new Map();
  singleTableShapes.set(tableName, tableShape);
  const lineHighlighter = (code: string) => {
    const splitLines = code.split("\n");
    const tableStartIndex =
      splitLines.findIndex((value) => value.trim().startsWith(tableName)) + 1;
    return {
      startLineNumber: tableStartIndex,
      endLineNumber: splitLines.length - 1,
    };
  };
  const codeTransformation = (code: string) =>
    insertOtherTablesComment(tableName, code);

  return (
    <ShowSchema
      activeSchema={activeSchema}
      inProgressSchema={inProgressSchema}
      shapes={singleTableShapes}
      hasShapeError={hadShapeError}
      showLearnMoreLink={false}
      lineHighlighter={lineHighlighter}
      codeTransformation={codeTransformation}
    />
  );
}

export function TableSchemaContainer({ tableName }: { tableName: string }) {
  const schemas = useQuery(udfs.getSchemas.default, {
    componentId: useNents().selectedNent?.id ?? null,
  });
  const { activeSchema, inProgressSchema } = useMemo(() => {
    if (!schemas) return {};

    return {
      activeSchema: parseAndFilterToSingleTable(tableName, schemas.active),
      inProgressSchema: parseAndFilterToSingleTable(
        tableName,
        schemas.inProgress,
      ),
    };
  }, [tableName, schemas]);
  const { tables, hadError: hadShapeError } = useTableShapes();
  return (
    <LoadingTransition>
      {tables && schemas && (
        <TableSchema
          tables={tables}
          tableName={tableName}
          activeSchema={activeSchema}
          inProgressSchema={inProgressSchema}
          hadShapeError={hadShapeError}
        />
      )}
    </LoadingTransition>
  );
}

export function topLevelFieldsForValidator(validator: Validator): {
  fields: Array<string>;
  areFieldsComplete: boolean;
} {
  if (validator.type === "object") {
    return {
      fields: ["_id", "_creationTime", ...Object.keys(validator.value)],
      areFieldsComplete: true,
    };
  }
  if (validator.type === "union") {
    const fields = new Set<string>();
    let areFieldsComplete = true;
    validator.value.forEach((v) => {
      const result = topLevelFieldsForValidator(v);
      result.fields.forEach((f) => fields.add(f));
      areFieldsComplete = areFieldsComplete && result.areFieldsComplete;
    });
    return { fields: Array.from(fields), areFieldsComplete };
  }
  return { fields: [], areFieldsComplete: false };
}
