import { Shape } from "shapes";
import { ValidatorJSON } from "convex/values";
import {
  SchemaJson,
  displaySchema,
  displaySchemaFromShapes,
  prettier,
} from "./format";

type ShapeTypes = Shape["type"];

const id: Shape = {
  type: "Id",
  tableName: "test",
};

const shape = (type: ShapeTypes, more = {}): Shape =>
  // @ts-ignore
  ({ type, ...more });

const field = (name: string, fieldShape: Shape, optional: boolean = false) => ({
  fieldName: name,
  shape: fieldShape,
  optional,
});

const oneFieldTable = (fieldShape: Shape) =>
  shape("Object", {
    fields: [field("_id", id), field("field1", fieldShape)],
  });

const allPrimitiveShapes = shape("Object", {
  fields: [
    field("unknown", shape("Unknown")),
    field("never", shape("Never")),
    field("id", id),
    field("null", shape("Null")),
    field("bigint", shape("Int64")),
    field("number", shape("Float64")),
    field("boolean", shape("Boolean")),
    field("string", shape("String")),
    field("bytes", shape("Bytes")),
  ],
});

const object = shape("Object", {
  fields: [
    field(
      "object",
      shape("Object", {
        fields: [
          field("field1", shape("Boolean")),
          field("field1", shape("String")),
        ],
      }),
    ),
  ],
});

const array = shape("Array", {
  shape: shape("Boolean"),
});

const union = shape("Union", {
  shapes: [shape("Boolean"), shape("String"), shape("Bytes")],
});

const record = shape("Record", {
  keyShape: shape("String"),
  valueShape: {
    optional: false,
    shape: shape("Float64"),
  },
});

const complicatedNestedShape = shape("Object", {
  fields: [
    field(
      "field1",
      shape("Object", {
        fields: [
          field(
            "field1",
            shape("Array", {
              shape: shape("Union", {
                shapes: [shape("Boolean"), shape("String")],
              }),
            }),
          ),
        ],
      }),
    ),
    field(
      "field2",
      shape("Array", {
        shape: shape("Union", {
          shapes: [shape("Array", { shape: shape("String") }), shape("String")],
        }),
      }),
    ),
  ],
});

// System properties shouldn't appear in the generated schema (they are added automatically).
const shapeWithOnlySystemProperties = shape("Object", {
  fields: [field("_id", id), field("_creationTime", shape("Float64"))],
});

// If the shape contains a nested object with fields that match system properties
// they should appear in the generated schema.
const shapeWithNestedSystemProperties = shape("Object", {
  fields: [
    field(
      "nested",
      shape("Object", {
        fields: [field("_id", id), field("_creationTime", shape("Float64"))],
      }),
    ),
  ],
});

// In a top-level union, system fields should still not appear in the schema.
const shapeWithTopLevelUnion = shape("Union", {
  shapes: [
    shape("Object", {
      fields: [
        field("_id", id),
        field("_creationTime", shape("Float64")),
        field("left", shape("String")),
      ],
    }),
    shape("Object", {
      fields: [
        field("_id", id),
        field("_creationTime", shape("Float64")),
        field("right", shape("String")),
      ],
    }),
  ],
});

const shapeWithOptionalTopLevelField = shape("Object", {
  fields: [field("optionalString", shape("String"), true)],
});

const shapeWithOptionalNestedField = shape("Object", {
  fields: [
    field(
      "object",
      shape("Object", {
        fields: [field("optionalString", shape("String"), true)],
      }),
    ),
  ],
});

describe("displaySchemaFromShapes", () => {
  test.each([
    {
      name: "one boolean field",
      schema: new Map([["table1", oneFieldTable(shape("Boolean"))]]),
    },
    {
      name: "two tables",
      schema: new Map([
        ["table1", oneFieldTable(shape("Boolean"))],
        ["table2", oneFieldTable(shape("Boolean"))],
      ]),
    },
    {
      name: "no schema on all tables",
      schema: new Map([
        ["table1", shape("Never")],
        ["table2", shape("Never")],
      ]),
    },
    {
      name: "primitives shapes",
      schema: new Map([["table1", oneFieldTable(allPrimitiveShapes)]]),
    },
    {
      name: "object shape",
      schema: new Map([["table1", oneFieldTable(object)]]),
    },
    {
      name: "array shape",
      schema: new Map([["table1", oneFieldTable(array)]]),
    },
    {
      name: "union shape",
      schema: new Map([["table1", oneFieldTable(union)]]),
    },
    {
      name: "record shape",
      schema: new Map([["table1", oneFieldTable(record)]]),
    },
    {
      name: "complicated shape",
      schema: new Map([["table1", oneFieldTable(complicatedNestedShape)]]),
    },
    {
      name: "shape with only system properties",
      schema: new Map([["table1", shapeWithOnlySystemProperties]]),
    },
    {
      name: "shape with nested system properties",
      schema: new Map([["table1", shapeWithNestedSystemProperties]]),
    },
    {
      name: "shape with top level union",
      schema: new Map([["table1", shapeWithTopLevelUnion]]),
    },
    {
      name: "shape with optional top level field",
      schema: new Map([["table1", shapeWithOptionalTopLevelField]]),
    },
    {
      name: "shape with optional nested field",
      schema: new Map([["table1", shapeWithOptionalNestedField]]),
    },
  ])("$name", ({ schema }) => {
    expect(displaySchemaFromShapes(schema)).toMatchSnapshot();
  });

  test("displaySchema with only search indexes", () => {
    const schemaJson: SchemaJson = {
      tables: [
        {
          tableName: "table",
          indexes: [],
          searchIndexes: [
            {
              indexDescriptor: "search_index",
              searchField: "property1",
              filterFields: [],
            },
          ],
          vectorIndexes: [],
          documentType: {
            type: "any",
          },
        },
        {
          tableName: "table_next",
          indexes: [],
          searchIndexes: [],
          vectorIndexes: [],
          documentType: {
            type: "any",
          },
        },
      ],
      schemaValidation: true,
    };
    expect(displaySchema(schemaJson)).toMatchSnapshot();
  });

  test("displaySchema with only vector indexes", () => {
    const schemaJson: SchemaJson = {
      tables: [
        {
          tableName: "table",
          indexes: [],
          searchIndexes: [],
          vectorIndexes: [
            {
              indexDescriptor: "vector_index",
              vectorField: "property1",
              dimensions: 1536,
              filterFields: [],
            },
          ],
          documentType: {
            type: "any",
          },
        },
        {
          tableName: "table_next",
          indexes: [],
          searchIndexes: [],
          vectorIndexes: [],
          documentType: {
            type: "any",
          },
        },
      ],
      schemaValidation: true,
    };
    expect(displaySchema(schemaJson)).toMatchSnapshot();
  });

  // Older schemas will not have a vector index field.
  test("displaySchema without vector indexes", () => {
    const schemaJson: SchemaJson = {
      tables: [
        {
          tableName: "table",
          indexes: [],
          searchIndexes: [],
          documentType: {
            type: "any",
          },
        } as any,
      ],
      schemaValidation: true,
    };
    expect(displaySchema(schemaJson)).toMatchSnapshot();
  });

  test("displaySchema", () => {
    const documentType: ValidatorJSON = {
      type: "object",
      value: {
        ref: {
          fieldType: { type: "id", tableName: "reference" },
          optional: false,
        },
        nullField: { fieldType: { type: "null" }, optional: false },
        numberField: { fieldType: { type: "number" }, optional: false },
        bigintField: { fieldType: { type: "bigint" }, optional: false },
        booleanField: { fieldType: { type: "boolean" }, optional: false },
        stringField: { fieldType: { type: "string" }, optional: false },
        bytesField: { fieldType: { type: "bytes" }, optional: false },
        arrayField: {
          fieldType: { type: "array", value: { type: "boolean" } },
          optional: false,
        },
        anyField: { fieldType: { type: "any" }, optional: false },
        literalBigint: {
          fieldType: {
            type: "literal",
            value: {
              $integer: "AQAAAAAAAAA=",
            },
          },
          optional: false,
        },
        literalNumber: {
          fieldType: {
            type: "literal",
            value: 0.0,
          },
          optional: false,
        },
        literalString: {
          fieldType: {
            type: "literal",
            value: "hello world",
          },
          optional: false,
        },
        literalBoolean: {
          fieldType: {
            type: "literal",
            value: true,
          },
          optional: false,
        },
        union: {
          fieldType: {
            type: "union",
            value: [{ type: "string" }, { type: "number" }],
          },
          optional: false,
        },
        object: {
          fieldType: {
            type: "object",
            value: {
              a: { fieldType: { type: "any" }, optional: true },
            },
          },
          optional: false,
        },
      },
    };
    const schemaJson: SchemaJson = {
      tables: [
        {
          tableName: "table",
          indexes: [
            { indexDescriptor: "by_a", fields: ["a"] },
            { indexDescriptor: "by_a_b", fields: ["a", "b"] },
          ],
          searchIndexes: [
            {
              indexDescriptor: "no_filter_fields",
              searchField: "property1",
              filterFields: [],
            },
            {
              indexDescriptor: "one_filter_field",
              searchField: "property1",
              filterFields: ["property1"],
            },
            {
              indexDescriptor: "two_filter_fields",
              searchField: "property1",
              filterFields: ["property1", "property2"],
            },
          ],
          vectorIndexes: [
            {
              indexDescriptor: "vector_no_filter_fields",
              vectorField: "property1",
              dimensions: 1536,
              filterFields: [],
            },
            {
              indexDescriptor: "vector_one_filter_field",
              vectorField: "property1",
              dimensions: 1536,
              filterFields: ["property1"],
            },
            {
              indexDescriptor: "vector_two_filter_fields",
              vectorField: "property1",
              dimensions: 1536,
              filterFields: ["property1", "property2"],
            },
          ],
          documentType,
        },
        {
          tableName: "table_any",
          indexes: [],
          searchIndexes: [],
          vectorIndexes: [],
          documentType: {
            type: "any",
          },
        },
        {
          tableName: "table_union",
          indexes: [],
          searchIndexes: [],
          vectorIndexes: [],
          documentType: {
            type: "union",
            value: [
              {
                type: "object",
                value: {
                  a: { fieldType: { type: "any" }, optional: false },
                  z: { fieldType: { type: "any" }, optional: true },
                },
              },
              {
                type: "object",
                value: {
                  b: { fieldType: { type: "any" }, optional: true },
                },
              },
            ],
          },
        },
      ],
      schemaValidation: true,
    };
    expect(displaySchema(schemaJson)).toMatchSnapshot();
  });
  test("schema validation false", () => {
    const schemaJson = { tables: [], schemaValidation: false };
    expect(displaySchema(schemaJson)).toMatchSnapshot();
  });
});

describe("prettier", () => {
  test("format succeeds", () => {
    const result = prettier("const a = 1;");
    expect(result).toBe("const a = 1;");
  });

  test("format does not include source code", () => {
    // This error for this code with a syntax error should not include the syntax error.
    const fn = () => {
      prettier("const a = ;");
    };
    expect(fn).toThrowError("Unexpected token (1:11)");
  });
});
