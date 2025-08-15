import { Shape } from "shapes";
import jsParser from "prettier-old-sync/parser-babel";
import standalone from "prettier-old-sync/standalone";
import { ObjectFieldType, ValidatorJSON, jsonToConvex } from "convex/values";
import { Index, SearchIndex, VectorIndex } from "convex/server";
import { format } from "date-fns";

type TableDefinition = {
  tableName: string;
  indexes: Index[];
  searchIndexes: SearchIndex[];
  vectorIndexes?: VectorIndex[];
  documentType: ValidatorJSON | null;
};

export type SchemaJson = {
  tables: TableDefinition[];
  schemaValidation: boolean;
};

export const prettier = (stmt: string, printWidth: number = 60) => {
  try {
    const prettierStmt: string = standalone.format(stmt, {
      plugins: [jsParser],
      printWidth,
      tabWidth: 2,
      semi: true,
      proseWrap: "always",
      parser: "babel-ts",
    });
    return prettierStmt.trim();
  } catch (e: any) {
    if (process.env.NEXT_PUBLIC_ENVIRONMENT === "development") {
      // It's safe to throw the whole error in development.
      throw e;
    }
    // Re-throw with only the first line of the prettier error, which describes the issue
    // Subsequent lines may include source code and should not be logged to our error tracking.
    throw new Error(e.message.split("\n")[0]);
  }
};

export function displaySchemaFromShape({
  shape,
  filterSystemFields = false,
  wrapObjects = true,
}: {
  shape: Shape;
  filterSystemFields?: boolean;
  wrapObjects?: boolean;
}): string {
  function objectFieldToSchema(p: { optional: boolean; shape: Shape }): string {
    if (p.optional) {
      return `v.optional(${displaySchemaFromShape({
        shape: p.shape,
      })})`;
    }
    return `${displaySchemaFromShape({
      shape: p.shape,
    })}`;
  }

  function fieldToSchema(p: {
    fieldName: string;
    optional: boolean;
    shape: Shape;
  }): string {
    return `"${p.fieldName}": ${objectFieldToSchema({
      optional: p.optional,
      shape: p.shape,
    })}`;
  }

  const variant = shape;

  switch (variant.type) {
    case "Array":
      return `v.array(${displaySchemaFromShape({
        shape: variant.shape,
      })})`;
    case "Boolean":
      return `v.boolean()`;
    case "Bytes":
      return `v.bytes()`;
    case "Float64":
      return `v.float64()`;
    case "Id":
      return `v.id("${variant.tableName}")`;
    case "Int64":
      return `v.int64()`;
    case "Map":
      return `v.map(${displaySchemaFromShape({
        shape: variant.keyShape,
      })}, ${displaySchemaFromShape({
        shape: variant.valueShape,
      })})`;
    case "Never":
      // If the developer saves an empty map, set, or array the shape of the values
      // is Never (we don't know it!). Convert to `v.any()` so the schema is at least functional.
      return `v.any()`;

    case "Null":
      return `v.null()`;
    case "Object": {
      const fieldsStr = filterSystemFields
        ? variant.fields
            // System fields like `_id` and `_creationTime` shouldn't be in schemas
            // (they are automatically added).
            .filter(
              (p) => p.fieldName !== "_id" && p.fieldName !== "_creationTime",
            )
            .map((p) => fieldToSchema(p))
            .join(",\n")
        : variant.fields.map((p) => fieldToSchema(p)).join(",\n");
      return wrapObjects ? `v.object({${fieldsStr}})` : `{${fieldsStr}}`;
    }
    case "Record": {
      const keys = displaySchemaFromShape({
        shape: variant.keyShape,
      });
      const values = objectFieldToSchema(variant.valueShape);
      return `v.record(${keys}, ${values})`;
    }
    case "Set":
      return `v.set(${displaySchemaFromShape({
        shape: variant.shape,
      })})`;
    case "String":
      return `v.string()`;
    case "Union":
      return `v.union(${variant.shapes
        // When we recurse, still consider this the root because we haven't actually
        // gone into nested objects yet!
        .map((variantShape) =>
          displaySchemaFromShape({
            shape: variantShape,
            filterSystemFields,
          }),
        )
        .join(",")})`;
    case "Unknown":
      return `v.any()`;
    default: {
      // Enforce that the switch-case is exhaustive.
      variant satisfies never;
      throw new Error(`Unrecognized variant ${shape.type} in ${shape}`);
    }
  }
}

function displayDocumentType(validator: ValidatorJSON): string {
  switch (validator.type) {
    case "object":
      return displayObjectSchema(validator.value);
    case "union":
      return displayValidator(validator);
    case "any":
      return displayValidator(validator);
    default:
      throw new Error("Top-level validator must be an object or union.");
  }
}

export function displayObjectFieldSchema(field: ObjectFieldType): string {
  const validator = displayValidator(field.fieldType);
  return field.optional ? `v.optional(${validator})` : validator;
}

function displayObjectSchema(object: Record<string, ObjectFieldType>): string {
  return `{${Object.keys(object)
    .map((key) => {
      const valueType = displayObjectFieldSchema(object[key]);
      return `${key}: ${valueType}`;
    })
    .join(",")}}`;
}

export function displayValidatorPretty(validator: ValidatorJSON): string {
  return prettier(displayValidator(validator));
}

function displayValidator(validator: ValidatorJSON): string {
  switch (validator.type) {
    case "null":
      return `v.null()`;
    case "number":
      return `v.float64()`;
    case "bigint":
      return `v.int64()`;
    case "boolean":
      return `v.boolean()`;
    case "string":
      return `v.string()`;
    case "bytes":
      return `v.bytes()`;
    case "any":
      return `v.any()`;
    case "literal":
      switch (typeof validator.value) {
        case "string":
          return `v.literal(${JSON.stringify(validator.value)})`;
        case "number":
          return `v.literal(${validator.value})`;
        case "boolean":
          return `v.literal(${validator.value})`;
        case "object":
          return `v.literal(${jsonToConvex(validator.value)}n)`;
        default: {
          throw new Error(
            `Unsupported literal type: ${typeof validator.value}`,
          );
        }
      }
    case "id":
      return `v.id("${validator.tableName}")`;
    case "array":
      return `v.array(${displayValidator(validator.value)})`;
    case "record":
      return `v.record(${displayValidator(
        validator.keys,
      )}, ${displayObjectFieldSchema(validator.values)})`;
    // Deprecated, but could be shown in History tab
    case "set" as any:
      return `v.set(${displayValidator((validator as any).value)})`;
    // Deprecated, but could be shown in History tab
    case "map" as any:
      return `v.map(${displayValidator(
        (validator as any).keys,
      )}, ${displayValidator((validator as any).values)})`;
    case "object":
      return `v.object(${displayObjectSchema(validator.value)})`;
    case "union":
      return `v.union(${validator.value
        .map((t: ValidatorJSON) => displayValidator(t))
        .join(",\n")})`;
    default: {
      // Enforce that the switch-case is exhaustive.
      validator satisfies never;
      throw new Error(`Unrecognized validator variant: ${validator}`);
    }
  }
}

function displayIndexes(indexes: Index[]): string {
  return indexes
    .map(
      (index) =>
        `.index("${index.indexDescriptor}", [${index.fields
          // Filter out system fields that start with underscore. Ideally these are filtered out in backend (CX-3805), but for now we do it here.
          .filter((field) => (field.length > 0 ? field[0] !== "_" : true))
          .map((field) => `"${field}"`)
          .join(",")}])`,
    )
    .join("");
}

function displaySearchIndexes(searchIndexes: SearchIndex[]): string {
  return searchIndexes
    .map(
      (searchIndex) =>
        `.searchIndex("${searchIndex.indexDescriptor}", {searchField: "${
          searchIndex.searchField
        }"
        ${appendFilterFieldsOrEmpty(searchIndex)}})`,
    )
    .join("");
}

function appendFilterFieldsOrEmpty(index: SearchIndex | VectorIndex): string {
  return index.filterFields.length > 0
    ? `, filterFields: [${index.filterFields
        .map((field) => `"${field}"`)
        .join(",")}]`
    : "";
}

function displayVectorIndexes(vectorIndexes: VectorIndex[]): string {
  return vectorIndexes
    .map(
      (vectorIndex) => `.vectorIndex("${vectorIndex.indexDescriptor}"
        , {vectorField: "${vectorIndex.vectorField}"
        , dimensions: ${vectorIndex.dimensions}
        ${appendFilterFieldsOrEmpty(vectorIndex)}
      })`,
    )
    .join("");
}

function displayTableDefinition(tableDefinition: TableDefinition): string {
  const documentType = displayDocumentType(
    tableDefinition.documentType ?? { type: "any" },
  );
  return `${tableDefinition.tableName}: defineTable(${documentType}
  )${displayIndexes(tableDefinition.indexes)}${displaySearchIndexes(
    tableDefinition.searchIndexes,
  )}${displayVectorIndexes(tableDefinition.vectorIndexes ?? [])}`;
}

export function displaySchema(schema: SchemaJson, relativePath = "") {
  const tables = schema.tables.map((table: any) =>
    displayTableDefinition(table),
  );
  return displayTableSchemas(tables, {
    schemaValidation: schema.schemaValidation,
    relativePath,
  });
}

export function displaySchemaFromShapes(
  shapes: Map<string, Shape>,
): string | undefined {
  const tables: string[] = [];
  shapes.forEach((shape, table) => {
    if (shape.type !== "Never") {
      const tableShape = displaySchemaFromShape({
        shape,
        filterSystemFields: true,
        wrapObjects: false,
      });
      tables.push(`"${table}": defineTable(${tableShape})`);
    }
  });

  return displayTableSchemas(tables, {
    schemaValidation: true,
    relativePath: "",
  });
}

function displayTableSchemas(
  tables: string[],
  options: {
    schemaValidation: boolean;
    relativePath: string;
  },
): string | undefined {
  if (!tables.length) {
    return undefined;
  }
  const imports = `
    import { defineSchema, defineTable } from "${options.relativePath}convex/server";
    import { v } from "${options.relativePath}convex/values"`;

  // schemaValidation is a default true flag, so we only make it false if it is explicitly set to false.
  // undefined => true.
  const schemaOptions =
    options.schemaValidation === false ? ", { schemaValidation: false }" : "";
  return prettier(`
    ${imports}

    export default defineSchema({${tables.join(",")}}${schemaOptions});`);
}

// Intl.NumberFormat uses B instead of G for billion prefix, so handroll
export function formatBytes(number: number): string;
export function formatBytes(number: number | null): string | null;
export function formatBytes(number: number | null) {
  const nonBreakingSpace = String.fromCharCode(160);
  if (number === null) {
    return null;
  }
  if (number === 0) {
    return `0${nonBreakingSpace}B`;
  }

  const k = 1024;
  const sizes = ["B", "KB", "MB", "GB", "TB", "PB", "EB", "ZB", "YB"];

  const i = Math.floor(Math.log(number) / Math.log(k));

  // parseFloat trick gets rid of trailing zeroes
  return `${parseFloat((number / k ** i).toFixed(2))}${nonBreakingSpace}${
    sizes[i]
  }`;
}

const NUMBER_FORMAT = new Intl.NumberFormat();
export function formatNumber(value: number): string;
export function formatNumber(value: number | null): string | null;
export function formatNumber(value: number | null): string | null {
  if (value === null) return null;
  return NUMBER_FORMAT.format(value);
}

const NUMBER_FORMAT_COMPACT = new Intl.NumberFormat("en-US", {
  notation: "compact",
  compactDisplay: "short",
});
export function formatNumberCompact(value: number | bigint): string;
export function formatNumberCompact(
  value: number | bigint | null,
): string | null;
export function formatNumberCompact(
  value: number | bigint | null,
): string | null {
  if (value === null) return null;
  return NUMBER_FORMAT_COMPACT.format(value);
}

export function msFormat(n: number): string {
  if (n > 1000) {
    let s = `${Math.floor(n / 100) / 10}`;
    if (!s.includes(".")) {
      s = `${s}.0`;
    }
    s = `${s}s`;
    return s;
  }
  return `${Math.floor(n)}ms`;
}

export function formatDateTime(date: Date): string {
  return format(date, "MMM dd, HH:mm:ss");
}

export function formatDate(date: Date): string {
  return format(date, "MMMM dd, yyyy");
}

export function toNumericUTC(dateString: string) {
  // Parsing manually the date to use UTC.
  const [year, month, day] = dateString.split("-");
  return Date.UTC(Number(year), Number(month) - 1, Number(day));
}

export const timeLabelForMinute = (value: string) => {
  if (!value) {
    return "";
  }
  // TODO(ari): Consolidate all the time rendering logic - this is a hack
  // for now
  if (value.includes("-") || !value.includes(":")) {
    return value;
  }
  const [time, modifier] = value.split(" ");
  const [hours, minutes] = time.split(":");
  const date = new Date();
  const hourValue = parseInt(hours);

  // Handle 12-hour to 24-hour conversion
  let hour24 = hourValue;
  if (modifier === "PM" && hourValue !== 12) {
    hour24 = hourValue + 12;
  } else if (modifier === "AM" && hourValue === 12) {
    hour24 = 0;
  }

  date.setHours(hour24);
  date.setMinutes(parseInt(minutes));
  const oneMinuteLater = new Date(date);
  oneMinuteLater.setMinutes(date.getMinutes() + 1);

  return `${formatTime(date)} – ${formatTime(oneMinuteLater)}`;
};

const formatTime = (date: Date) => {
  let hours = date.getHours();
  const minutes = date.getMinutes();
  const ampm = hours >= 12 ? "PM" : "AM";
  hours %= 12;
  hours = hours || 12; // the hour '0' should be '12'
  const strMinutes = minutes < 10 ? `0${minutes}` : minutes;
  return `${hours}:${strMinutes} ${ampm}`;
};
