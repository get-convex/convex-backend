import { ValidatorJSON, ObjectFieldType } from "convex/values";
import { Shape } from "shapes";
import { SchemaJson, prettier } from "@common/lib/format";

/**
 * A single top-level field of a table, summarized for display in the schema
 * visualizer. `references` lists the tables that this field points at via
 * `v.id("...")` (directly, or nested inside arrays/records/unions/objects).
 */
export type SchemaField = {
  name: string;
  // A compact, human-readable type label, e.g. `Id<users>`, `string`,
  // `Id<messages>[]` or `{ … }`.
  type: string;
  // The full TypeScript-style type with object bodies expanded (e.g.
  // `{ theme: "light" | "dark" }`), present only when the compact label hides
  // detail (i.e. it collapsed an object to `{ … }`). Used to reveal the
  // complete type when a field is expanded in the side panel.
  fullType?: string;
  optional: boolean;
  references: string[];
};

/**
 * An index defined on a table. `kind` distinguishes database indexes from
 * search and vector indexes. `fields` is the indexed field list (for search and
 * vector indexes, the search/vector field followed by any filter fields).
 */
export type SchemaIndex = {
  name: string;
  kind: "database" | "search" | "vector";
  fields: string[];
};

export type SchemaNode = {
  table: string;
  fields: SchemaField[];
  // Indexes defined on the table. Only available from a saved schema (shapes
  // can't infer indexes), so this is empty for shape-derived graphs.
  indexes: SchemaIndex[];
  // True when the table exists in the deployment's data but isn't declared in
  // the saved schema (its fields are inferred from shapes). Always false for
  // pure shape-derived graphs — without a schema, there's nothing to be missing
  // from.
  notInSchema: boolean;
};

export type SchemaEdge = {
  id: string;
  source: string;
  target: string;
  field: string;
  optional: boolean;
};

export type SchemaGraph = {
  nodes: SchemaNode[];
  edges: SchemaEdge[];
};

// --- Validator (saved schema) helpers ---------------------------------------

function collectReferencesFromValidator(
  validator: ValidatorJSON,
  acc: Set<string>,
): void {
  switch (validator.type) {
    case "id":
      acc.add(validator.tableName);
      break;
    case "array":
      collectReferencesFromValidator(validator.value, acc);
      break;
    case "record":
      collectReferencesFromValidator(validator.values.fieldType, acc);
      break;
    case "object":
      Object.values(validator.value).forEach((field) =>
        collectReferencesFromValidator(field.fieldType, acc),
      );
      break;
    case "union":
      validator.value.forEach((v) => collectReferencesFromValidator(v, acc));
      break;
    default:
      break;
  }
}

// A TypeScript-style type label for a validator. By default objects collapse to
// `{ … }` to stay compact inside a graph node; pass `expand` to render the full
// object body (e.g. `{ name: string; theme: "light" | "dark" }`).
function validatorToLabel(validator: ValidatorJSON, expand = false): string {
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
      return "bytes";
    case "any":
      return "any";
    case "literal":
      return typeof validator.value === "string"
        ? `"${validator.value}"`
        : String(validator.value);
    case "id":
      return `Id<"${validator.tableName}">`;
    case "array":
      return `${validatorToLabel(validator.value, expand)}[]`;
    case "record":
      return `Record<${validatorToLabel(
        validator.keys,
        expand,
      )}, ${validatorToLabel(validator.values.fieldType, expand)}>`;
    case "object": {
      if (!expand) {
        return "{ … }";
      }
      const entries = Object.entries(validator.value);
      if (entries.length === 0) {
        return "{}";
      }
      return `{ ${entries
        .map(
          ([name, field]) =>
            `${name}${field.optional ? "?" : ""}: ${validatorToLabel(
              field.fieldType,
              true,
            )}`,
        )
        .join("; ")} }`;
    }
    case "union":
      return validator.value
        .map((v) => validatorToLabel(v, expand))
        .join(" | ");
    default:
      return "any";
  }
}

// Pretty-print a TypeScript type expression by formatting it as a type alias
// (so the parser accepts the bare type) and stripping the wrapper. Falls back
// to the unformatted type if formatting fails.
function prettifyType(type: string): string {
  try {
    return prettier(`type _ = ${type}`, 40)
      .replace(/^type _ =\s*/, "")
      .replace(/;\s*$/, "")
      .trim();
  } catch {
    return type;
  }
}

function fieldsFromObjectValidator(
  fields: Record<string, ObjectFieldType>,
): SchemaField[] {
  return Object.entries(fields).map(([name, field]) => {
    const references = new Set<string>();
    collectReferencesFromValidator(field.fieldType, references);
    const type = validatorToLabel(field.fieldType);
    return {
      name,
      type,
      // Only carry the full form when the compact label collapsed something.
      fullType: type.includes("…")
        ? prettifyType(validatorToLabel(field.fieldType, true))
        : undefined,
      optional: field.optional,
      references: Array.from(references),
    };
  });
}

function fieldsFromDocumentType(
  documentType: ValidatorJSON | null,
): SchemaField[] {
  if (!documentType) {
    return [];
  }
  if (documentType.type === "object") {
    return fieldsFromObjectValidator(documentType.value);
  }
  if (documentType.type === "union") {
    // Merge fields across all union members, keeping the first occurrence of
    // each field name. A field present in only some members is marked optional.
    const merged = new Map<string, SchemaField>();
    const fieldCounts = new Map<string, number>();
    documentType.value.forEach((member) => {
      const memberFields =
        member.type === "object" ? fieldsFromObjectValidator(member.value) : [];
      memberFields.forEach((field) => {
        fieldCounts.set(field.name, (fieldCounts.get(field.name) ?? 0) + 1);
        if (!merged.has(field.name)) {
          merged.set(field.name, field);
        }
      });
    });
    return Array.from(merged.values()).map((field) => ({
      ...field,
      optional:
        field.optional ||
        (fieldCounts.get(field.name) ?? 0) < documentType.value.length,
    }));
  }
  return [];
}

// --- Shape (inferred schema) helpers ----------------------------------------

function collectReferencesFromShape(shape: Shape, acc: Set<string>): void {
  switch (shape.type) {
    case "Id":
      acc.add(shape.tableName);
      break;
    case "Array":
      collectReferencesFromShape(shape.shape, acc);
      break;
    case "Record":
      collectReferencesFromShape(shape.valueShape.shape, acc);
      break;
    case "Object":
      shape.fields.forEach((f) => collectReferencesFromShape(f.shape, acc));
      break;
    case "Union":
      shape.shapes.forEach((s) => collectReferencesFromShape(s, acc));
      break;
    default:
      break;
  }
}

function shapeToLabel(shape: Shape, expand = false): string {
  switch (shape.type) {
    case "Id":
      return `Id<"${shape.tableName}">`;
    case "Array":
      return `${shapeToLabel(shape.shape, expand)}[]`;
    case "Record":
      return `Record<${shapeToLabel(shape.keyShape, expand)}, ${shapeToLabel(
        shape.valueShape.shape,
        expand,
      )}>`;
    case "Object": {
      if (!expand) {
        return "{ … }";
      }
      if (shape.fields.length === 0) {
        return "{}";
      }
      return `{ ${shape.fields
        .map(
          (f) =>
            `${f.fieldName}${f.optional ? "?" : ""}: ${shapeToLabel(
              f.shape,
              true,
            )}`,
        )
        .join("; ")} }`;
    }
    case "Union":
      return shape.shapes.map((s) => shapeToLabel(s, expand)).join(" | ");
    case "Boolean":
      return "boolean";
    case "Bytes":
      return "bytes";
    case "Float64":
      return "number";
    case "Int64":
      return "bigint";
    case "Null":
      return "null";
    case "String":
      return "string";
    case "Never":
      return "never";
    default:
      return "unknown";
  }
}

function fieldsFromShape(shape: Shape): SchemaField[] {
  if (shape.type === "Object") {
    return shape.fields
      .filter((f) => f.fieldName !== "_id" && f.fieldName !== "_creationTime")
      .map((f) => {
        const references = new Set<string>();
        collectReferencesFromShape(f.shape, references);
        const type = shapeToLabel(f.shape);
        return {
          name: f.fieldName,
          type,
          // Only carry the full form when the compact label collapsed something.
          fullType: type.includes("…")
            ? prettifyType(shapeToLabel(f.shape, true))
            : undefined,
          optional: f.optional,
          references: Array.from(references),
        };
      });
  }
  return [];
}

// --- Graph assembly ----------------------------------------------------------

function assembleGraph(
  rawNodes: {
    table: string;
    fields: SchemaField[];
    indexes: SchemaIndex[];
    notInSchema?: boolean;
  }[],
): SchemaGraph {
  const tableNames = new Set(rawNodes.map((n) => n.table));
  const edges: SchemaEdge[] = [];
  const seenEdges = new Set<string>();

  rawNodes.forEach((node) => {
    node.fields.forEach((field) => {
      field.references.forEach((target) => {
        const id = `${node.table}.${field.name}->${target}`;
        if (seenEdges.has(id)) {
          return;
        }
        seenEdges.add(id);
        // Only draw edges to tables that exist in this schema/component.
        if (tableNames.has(target)) {
          edges.push({
            id,
            source: node.table,
            target,
            field: field.name,
            optional: field.optional,
          });
        }
      });
    });
  });

  const nodes: SchemaNode[] = rawNodes.map((node) => ({
    table: node.table,
    fields: node.fields,
    indexes: node.indexes,
    notInSchema: node.notInSchema ?? false,
  }));

  return { nodes, edges };
}

// Flatten a table's database, search, and vector indexes into a single list.
function indexesFromTable(table: SchemaJson["tables"][number]): SchemaIndex[] {
  return [
    ...(table.indexes ?? []).map(
      (index): SchemaIndex => ({
        name: index.indexDescriptor,
        kind: "database",
        fields: index.fields,
      }),
    ),
    ...(table.searchIndexes ?? []).map(
      (index): SchemaIndex => ({
        name: index.indexDescriptor,
        kind: "search",
        fields: [index.searchField, ...(index.filterFields ?? [])],
      }),
    ),
    ...(table.vectorIndexes ?? []).map(
      (index): SchemaIndex => ({
        name: index.indexDescriptor,
        kind: "vector",
        fields: [index.vectorField, ...(index.filterFields ?? [])],
      }),
    ),
  ];
}

/**
 * Build a relationship graph from a saved schema. When `shapes` is provided,
 * tables that exist in the data but aren't declared in the schema are merged in
 * (with fields inferred from their shape) and flagged `notInSchema` so the UI
 * can mark them. This keeps the diagram a faithful view of everything in the
 * deployment, not just the declared tables.
 */
export function buildGraphFromSchema(
  schema: SchemaJson,
  shapes?: Map<string, Shape>,
): SchemaGraph {
  const schemaTableNames = new Set(schema.tables.map((t) => t.tableName));
  const rawNodes: {
    table: string;
    fields: SchemaField[];
    indexes: SchemaIndex[];
    notInSchema?: boolean;
  }[] = schema.tables.map((table) => ({
    table: table.tableName,
    fields: fieldsFromDocumentType(table.documentType),
    indexes: indexesFromTable(table),
  }));

  if (shapes) {
    shapes.forEach((shape, table) => {
      // Skip tables already declared in the schema; everything else in the data
      // is a not-in-schema table. Empty tables come back with a `Never` shape
      // (and no inferable fields) — still include them so the diagram matches
      // the data view's table list, which shows empty tables too.
      if (schemaTableNames.has(table)) {
        return;
      }
      rawNodes.push({
        table,
        fields: fieldsFromShape(shape),
        indexes: [],
        notInSchema: true,
      });
    });
  }

  rawNodes.sort((a, b) => a.table.localeCompare(b.table));
  return assembleGraph(rawNodes);
}

/**
 * Build a relationship graph from shapes inferred from the data. Includes every
 * table the data view shows — empty tables come back with a `Never` shape (and
 * no inferable fields) but are still rendered as nodes so the diagram matches
 * the data view's table list.
 */
export function buildGraphFromShapes(shapes: Map<string, Shape>): SchemaGraph {
  const rawNodes = Array.from(shapes.entries())
    .map(([table, shape]) => ({
      table,
      fields: fieldsFromShape(shape),
      indexes: [] as SchemaIndex[],
    }))
    .sort((a, b) => a.table.localeCompare(b.table));
  return assembleGraph(rawNodes);
}
