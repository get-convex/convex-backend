/* eslint-disable @typescript-eslint/no-unused-vars */
import { GenericId } from "../values/index.js";
import { describe, expect, test } from "vitest";
import { assert, Equals } from "../test/type_testing.js";
import { SystemIndexes } from "./system_fields.js";
import {
  defineSchema,
  defineTable,
  DataModelFromSchemaDefinition,
  type FlowExpr,
  type FlowFieldFilter,
} from "./schema.js";
import { v, Infer } from "../values/validator.js";

describe("DataModelFromSchemaDefinition", () => {
  test("defineSchema produces the correct data model for basic types", () => {
    const schema = defineSchema({
      table: defineTable({
        ref: v.id("reference"),
        null: v.null(),
        number: v.number(),
        float64: v.float64(),
        int64: v.int64(),
        boolean: v.boolean(),
        string: v.string(),
        bytes: v.bytes(),
        array: v.array(v.boolean()),
        record: v.record(v.string(), v.boolean()),
      }),
    });
    type DataModel = DataModelFromSchemaDefinition<typeof schema>;
    type ExpectedDocument = {
      _id: GenericId<"table">;
      _creationTime: number;
      ref: GenericId<"reference">;
      null: null;
      number: number;
      float64: number;
      int64: bigint;
      boolean: boolean;
      string: string;
      array: boolean[];
      bytes: ArrayBuffer;
      record: Record<string, boolean>;
    };
    type ExpectedFieldPaths =
      | "_id"
      | "_creationTime"
      | "ref"
      | "null"
      | "number"
      | "float64"
      | "int64"
      | "boolean"
      | "string"
      | "bytes"
      | "array"
      | "record"
      | `record.${string}`;

    type ExpectedDataModel = {
      table: {
        document: ExpectedDocument;
        fieldPaths: ExpectedFieldPaths;
        indexes: SystemIndexes;
        searchIndexes: {};
        vectorIndexes: {};
        computedFields: never;
      };
    };

    assert<Equals<DataModel, ExpectedDataModel>>();
  });
  test("defineSchema produces the correct data model any", () => {
    const schema = defineSchema({
      table: defineTable({
        any: v.any(),
      }),
    });
    type DataModel = DataModelFromSchemaDefinition<typeof schema>;
    type ExpectedDocument = {
      _id: GenericId<"table">;
      _creationTime: number;
      any: any;
    };
    type ExpectedFieldPaths =
      | "_id"
      | "_creationTime"
      | "any"
      // You can index anything into an `any`
      | `any.${string}`;

    type ExpectedDataModel = {
      table: {
        document: ExpectedDocument;
        fieldPaths: ExpectedFieldPaths;
        indexes: SystemIndexes;
        searchIndexes: {};
        vectorIndexes: {};
        computedFields: never;
      };
    };

    assert<Equals<DataModel, ExpectedDataModel>>();
  });
  test("defineSchema handles all the literal types", () => {
    const schema = defineSchema({
      table: defineTable({
        string: v.literal("string"),
        number: v.literal(1),
        bigint: v.literal(1n),
        boolean: v.literal(true),
      }),
    });
    type DataModel = DataModelFromSchemaDefinition<typeof schema>;
    type ExpectedDocument = {
      _id: GenericId<"table">;
      _creationTime: number;
      string: "string";
      number: 1;
      bigint: 1n;
      boolean: true;
    };
    type ExpectedFieldPaths =
      | "_id"
      | "_creationTime"
      | "string"
      | "number"
      | "bigint"
      | "boolean";

    type ExpectedDataModel = {
      table: {
        document: ExpectedDocument;
        fieldPaths: ExpectedFieldPaths;
        indexes: SystemIndexes;
        searchIndexes: {};
        vectorIndexes: {};
        computedFields: never;
      };
    };
    assert<Equals<DataModel, ExpectedDataModel>>();
  });

  test("defineSchema handles nested objects", () => {
    const schema = defineSchema({
      table: defineTable({
        prop1: v.string(),
        nested: v.object({
          prop2: v.string(),
          doublyNested: v.object({
            prop3: v.string(),
          }),
        }),
      }),
    });
    type DataModel = DataModelFromSchemaDefinition<typeof schema>;
    type ExpectedDocument = {
      _id: GenericId<"table">;
      _creationTime: number;
      prop1: string;
      nested: {
        prop2: string;
        doublyNested: {
          prop3: string;
        };
      };
    };
    type ExpectedFieldPaths =
      | "_id"
      | "_creationTime"
      | "prop1"
      | "nested"
      | "nested.prop2"
      | "nested.doublyNested"
      | "nested.doublyNested.prop3";

    type ExpectedDataModel = {
      table: {
        document: ExpectedDocument;
        fieldPaths: ExpectedFieldPaths;
        indexes: SystemIndexes;
        searchIndexes: {};
        vectorIndexes: {};
        computedFields: never;
      };
    };
    assert<Equals<DataModel, ExpectedDataModel>>();
  });

  test("defineSchema handles object unions", () => {
    const schema = defineSchema({
      table: defineTable(
        v.union(
          v.object({
            string: v.string(),
          }),
          v.object({
            number: v.number(),
          }),
        ),
      ),
    });
    type DataModel = DataModelFromSchemaDefinition<typeof schema>;
    type ExpectedDocument =
      | {
          _id: GenericId<"table">;
          _creationTime: number;
          string: string;
        }
      | {
          _id: GenericId<"table">;
          _creationTime: number;
          number: number;
        };
    type ExpectedFieldPaths = "_id" | "_creationTime" | "string" | "number";
    type ExpectedDataModel = {
      table: {
        document: ExpectedDocument;
        fieldPaths: ExpectedFieldPaths;
        indexes: SystemIndexes;
        searchIndexes: {};
        vectorIndexes: {};
        computedFields: never;
      };
    };
    assert<Equals<DataModel, ExpectedDataModel>>();
  });

  test("defineSchema handles mixed unions", () => {
    // Testing that we can mix objects and other things in unions and still
    // generate the right index field paths.
    const schema = defineSchema({
      table: defineTable({
        property: v.union(v.object({ string: v.string() }), v.number()),
      }),
    });
    type DataModel = DataModelFromSchemaDefinition<typeof schema>;
    type ExpectedDocument = {
      _id: GenericId<"table">;
      _creationTime: number;
      property: { string: string } | number;
    };
    type ExpectedFieldPaths =
      | "_id"
      | "_creationTime"
      | "property"
      | "property.string";
    type ExpectedDataModel = {
      table: {
        document: ExpectedDocument;
        fieldPaths: ExpectedFieldPaths;
        indexes: SystemIndexes;
        searchIndexes: {};
        vectorIndexes: {};
        computedFields: never;
      };
    };
    assert<Equals<DataModel, ExpectedDataModel>>();
  });

  test("defineSchema handles array of unions", () => {
    const schema = defineSchema({
      table: defineTable({
        property: v.array(v.union(v.number(), v.string())),
      }),
    });
    type DataModel = DataModelFromSchemaDefinition<typeof schema>;
    type ExpectedDocument = {
      _id: GenericId<"table">;
      _creationTime: number;
      property: (number | string)[];
    };
    type ExpectedFieldPaths = "_id" | "_creationTime" | "property";
    type ExpectedDataModel = {
      table: {
        document: ExpectedDocument;
        fieldPaths: ExpectedFieldPaths;
        indexes: SystemIndexes;
        searchIndexes: {};
        vectorIndexes: {};
        computedFields: never;
      };
    };
    assert<Equals<DataModel, ExpectedDataModel>>();
  });

  test("defineSchema handles records with Ids", () => {
    const schema = defineSchema({
      table: defineTable({
        property: v.record(v.id("reference"), v.string()),
      }),
    });
    type DataModel = DataModelFromSchemaDefinition<typeof schema>;
    type ExpectedDocument = {
      _id: GenericId<"table">;
      _creationTime: number;
      property: Record<GenericId<"reference">, string>;
    };
    type ExpectedFieldPaths =
      | "_id"
      | "_creationTime"
      | "property"
      | `property.${string}`;
    type ExpectedDataModel = {
      table: {
        document: ExpectedDocument;
        fieldPaths: ExpectedFieldPaths;
        indexes: SystemIndexes;
        searchIndexes: {};
        vectorIndexes: {};
        computedFields: never;
      };
    };
    assert<Equals<DataModel, ExpectedDataModel>>();
  });

  test("defineSchema handles records with type unions", () => {
    const schema = defineSchema({
      table: defineTable({
        property: v.record(v.union(v.id("foo"), v.id("bla")), v.string()),
      }),
    });
    type DataModel = DataModelFromSchemaDefinition<typeof schema>;
    type ExpectedDocument = {
      _id: GenericId<"table">;
      _creationTime: number;
      property: Record<GenericId<"foo"> | GenericId<"bla">, string>;
    };
    type ExpectedFieldPaths =
      | "_id"
      | "_creationTime"
      | "property"
      | `property.${string}`;
    type ExpectedDataModel = {
      table: {
        document: ExpectedDocument;
        fieldPaths: ExpectedFieldPaths;
        indexes: SystemIndexes;
        searchIndexes: {};
        vectorIndexes: {};
        computedFields: never;
      };
    };
    assert<Equals<DataModel, ExpectedDataModel>>();
  });

  test("defineSchema handles optional keys", () => {
    const schema = defineSchema({
      table: defineTable({
        required: v.string(),
        optional: v.optional(v.boolean()),
        nested: v.object({
          required: v.int64(),
          optional: v.optional(v.number()),
        }),
      }),
    });
    type DataModel = DataModelFromSchemaDefinition<typeof schema>;
    type ExpectedDocument = {
      _id: GenericId<"table">;
      _creationTime: number;
      required: string;
      optional?: boolean;
      nested: {
        required: bigint;
        optional?: number;
      };
    };
    type ExpectedFieldPaths =
      | "_id"
      | "_creationTime"
      | "required"
      | "optional"
      | "nested"
      | "nested.required"
      | "nested.optional";

    type ExpectedDataModel = {
      table: {
        document: ExpectedDocument;
        fieldPaths: ExpectedFieldPaths;
        indexes: SystemIndexes;
        searchIndexes: {};
        vectorIndexes: {};
        computedFields: never;
      };
    };
    assert<Equals<DataModel, ExpectedDataModel>>();
  });

  test("defineSchema supports loose schemas", () => {
    const schema = defineSchema(
      {
        table: defineTable({
          property: v.string(),
        }),
      },
      { strictTableNameTypes: false },
    );

    type DataModel = DataModelFromSchemaDefinition<typeof schema>;
    type ExpectedDocument = {
      _id: GenericId<"table">;
      _creationTime: number;
      property: string;
    };

    type ExpectedDataModel = {
      table: {
        document: ExpectedDocument;
        fieldPaths: "_id" | "_creationTime" | "property";
        indexes: SystemIndexes;
        searchIndexes: {};
        vectorIndexes: {};
        computedFields: never;
      };
      [tableName: string]: {
        document: any;
        fieldPaths: string;
        indexes: {};
        searchIndexes: {};
        vectorIndexes: {};
        computedFields: string;
      };
    };
    assert<Equals<DataModel, ExpectedDataModel>>();
  });

  test("defineSchema generates index types", () => {
    const schema = defineSchema({
      table: defineTable({
        property1: v.string(),
        property2: v.string(),
      })
        .index("by_property1", ["property1"])
        .index("by_property1_property2", ["property1", "property2"]),
    });
    type DataModel = DataModelFromSchemaDefinition<typeof schema>;
    type ExpectedDocument = {
      _id: GenericId<"table">;
      _creationTime: number;
      property1: string;
      property2: string;
    };
    type ExpectedFieldPaths =
      | "_id"
      | "_creationTime"
      | "property1"
      | "property2";
    type ExpectedIndexes = {
      by_property1: ["property1", "_creationTime"];
      by_property1_property2: ["property1", "property2", "_creationTime"];

      // System indexes
      by_creation_time: ["_creationTime"];
      by_id: ["_id"];
    };
    type ExpectedDataModel = {
      table: {
        document: ExpectedDocument;
        fieldPaths: ExpectedFieldPaths;
        indexes: ExpectedIndexes;
        searchIndexes: {};
        vectorIndexes: {};
        computedFields: never;
      };
    };
    assert<Equals<DataModel, ExpectedDataModel>>();
  });

  test("defineSchema creates staged indexes", () => {
    const schema = defineSchema({
      table: defineTable({
        enabled: v.string(),
        enabled2: v.string(),
        enabled3: v.string(),
        staged: v.string(),
      })
        .index("by_enabled", ["enabled"])
        .index("by_enabled2", { fields: ["enabled2"] })
        .index("by_enabled3", { fields: ["enabled3"], staged: false })
        .index("by_staged", { fields: ["staged"], staged: true }),
    });
    type Indexes = DataModelFromSchemaDefinition<
      typeof schema
    >["table"]["indexes"];
    type ExpectedIndexes = {
      by_enabled: ["enabled", "_creationTime"];
      by_enabled2: ["enabled2", "_creationTime"];
      by_enabled3: ["enabled3", "_creationTime"];
      by_id: ["_id"];
      by_creation_time: ["_creationTime"];
    };
    assert<Equals<Indexes, ExpectedIndexes>>();
  });

  test("defineSchema creates staged search indexes", () => {
    const schema = defineSchema({
      table: defineTable({
        enabled: v.string(),
        enabled2: v.string(),
        staged: v.string(),
      })
        .searchIndex("by_enabled", { searchField: "enabled" })
        .searchIndex("by_enabled2", { searchField: "enabled2", staged: false })
        .searchIndex("by_staged", { searchField: "staged", staged: true }),
    });
    type SearchIndexes = DataModelFromSchemaDefinition<
      typeof schema
    >["table"]["searchIndexes"];
    type ExpectedSearchIndexes = {
      by_enabled: {
        searchField: "enabled";
        filterFields: never;
      };
      by_enabled2: {
        searchField: "enabled2";
        filterFields: never;
      };
    };
    assert<Equals<SearchIndexes, ExpectedSearchIndexes>>();
  });

  test("defineSchema creates staged vector indexes", () => {
    const schema = defineSchema({
      table: defineTable({
        enabled: v.string(),
        enabled2: v.string(),
        staged: v.string(),
      })
        .vectorIndex("by_enabled", {
          vectorField: "enabled",
          dimensions: 1536,
        })
        .vectorIndex("by_enabled2", {
          vectorField: "enabled2",
          dimensions: 1536,
          staged: false,
        })
        .vectorIndex("by_staged", {
          vectorField: "staged",
          dimensions: 1536,
          staged: true,
        }),
    });
    type VectorIndexes = DataModelFromSchemaDefinition<
      typeof schema
    >["table"]["vectorIndexes"];
    type ExpectedVectorIndexes = {
      by_enabled: {
        vectorField: "enabled";
        dimensions: number;
        filterFields: never;
      };
      by_enabled2: {
        vectorField: "enabled2";
        dimensions: number;
        filterFields: never;
      };
    };
    assert<Equals<VectorIndexes, ExpectedVectorIndexes>>();
  });
});

test("defineSchema doesn’t allow creating indexes with a staged status not known at compile time", () => {
  defineSchema({
    // @ts-expect-error
    table: defineTable({
      field: v.string(),
    }).index("staged_database_index", {
      fields: ["field"],
      staged: Math.random() < 0.5,
    }),
  });

  defineSchema({
    table: defineTable({
      field: v.string(),
    }).searchIndex("staged_search_index", {
      searchField: "field",
      // @ts-expect-error
      staged: Math.random() < 0.5,
    }),
  });

  defineSchema({
    table: defineTable({
      field: v.array(v.float64()),
    }).vectorIndex("staged_vector_index", {
      vectorField: "field",
      dimensions: 42,
      // @ts-expect-error
      staged: Math.random() < 0.5,
    }),
  });
});

test("defineSchema generates search index types", () => {
  const schema = defineSchema({
    table: defineTable({
      property1: v.string(),
      property2: v.string(),
    })
      .searchIndex("no_filter_fields", {
        searchField: "property1",
      })
      .searchIndex("one_filter_field", {
        searchField: "property1",
        filterFields: ["property1"],
      })
      .searchIndex("two_filter_fields", {
        searchField: "property1",
        filterFields: ["property1", "property2"],
      }),
  });
  type DataModel = DataModelFromSchemaDefinition<typeof schema>;
  type ExpectedDocument = {
    _id: GenericId<"table">;
    _creationTime: number;
    property1: string;
    property2: string;
  };
  type ExpectedFieldPaths = "_id" | "_creationTime" | "property1" | "property2";
  type ExpectedSearchIndexes = {
    no_filter_fields: {
      searchField: "property1";
      filterFields: never;
    };
    one_filter_field: {
      searchField: "property1";
      filterFields: "property1";
    };
    two_filter_fields: {
      searchField: "property1";
      filterFields: "property1" | "property2";
    };
  };
  type ExpectedDataModel = {
    table: {
      document: ExpectedDocument;
      fieldPaths: ExpectedFieldPaths;
      indexes: SystemIndexes;
      searchIndexes: ExpectedSearchIndexes;
      vectorIndexes: {};
      computedFields: never;
    };
  };
  assert<Equals<DataModel, ExpectedDataModel>>();
});

test("defineSchema generates vector search index types", () => {
  const schema = defineSchema({
    table: defineTable({
      property1: v.string(),
      property2: v.string(),
      embedding: v.array(v.float64()),
    })
      .vectorIndex("no_filter_fields", {
        vectorField: "embedding",
        dimensions: 1536,
      })
      .vectorIndex("one_filter_field", {
        vectorField: "embedding",
        dimensions: 1536,
        filterFields: ["property1"],
      })
      .vectorIndex("two_filter_fields", {
        vectorField: "embedding",
        dimensions: 1536,
        filterFields: ["property1", "property2"],
      }),
  });
  type DataModel = DataModelFromSchemaDefinition<typeof schema>;
  type ExpectedDocument = {
    _id: GenericId<"table">;
    _creationTime: number;
    property1: string;
    property2: string;
    embedding: number[];
  };
  type ExpectedFieldPaths =
    | "_id"
    | "_creationTime"
    | "property1"
    | "property2"
    | "embedding";
  type ExpectedVectorSearchIndexes = {
    no_filter_fields: {
      vectorField: "embedding";
      dimensions: number;
      filterFields: never;
    };
    one_filter_field: {
      vectorField: "embedding";
      dimensions: number;
      filterFields: "property1";
    };
    two_filter_fields: {
      vectorField: "embedding";
      dimensions: number;
      filterFields: "property1" | "property2";
    };
  };
  schema.tables.table.vectorIndex;
  type ExpectedDataModel = {
    table: {
      document: ExpectedDocument;
      fieldPaths: ExpectedFieldPaths;
      indexes: SystemIndexes;
      searchIndexes: {};
      vectorIndexes: ExpectedVectorSearchIndexes;
      computedFields: never;
    };
  };
  assert<Equals<DataModel, ExpectedDataModel>>();
});

test("defineTable collects indexes", () => {
  const table = defineTable({
    a: v.string(),
    b: v.string(),
  })
    .index("by_a", ["a"])
    .index("by_a_b", ["a", "b"]);

  expect(table.export().indexes).toEqual([
    { indexDescriptor: "by_a", fields: ["a"] },
    { indexDescriptor: "by_a_b", fields: ["a", "b"] },
  ]);
});

test("Experimental API table.[' indexes']() returns indexes", () => {
  const table = defineTable({
    a: v.string(),
    b: v.string(),
  })
    .index("by_a", ["a"])
    .index("by_a_b", ["a", "b"]);

  expect(table[" indexes"]()).toEqual([
    { indexDescriptor: "by_a", fields: ["a"] },
    { indexDescriptor: "by_a_b", fields: ["a", "b"] },
  ]);
});

describe("JsonTypesFromSchema", () => {
  test("TableDefinition includes field types", () => {
    const table = defineTable({
      ref: v.id("reference"),
      nullField: v.null(),
      numberField: v.number(),
      float64Field: v.float64(),
      int64Field: v.int64(),
      booleanField: v.boolean(),
      stringField: v.string(),
      bytesField: v.bytes(),
      arrayField: v.array(v.boolean()),
      anyField: v.any(),
      literalBigint: v.literal(1n),
      literalNumber: v.literal(0.0),
      literalString: v.literal("hello world"),
      literalBoolean: v.literal(true),
      union: v.union(v.string(), v.number()),
      object: v.object({ a: v.optional(v.any()) }),
    }).export();
    expect(table.documentType).toEqual({
      type: "object",
      value: {
        ref: {
          fieldType: { type: "id", tableName: "reference" },
          optional: false,
        },
        nullField: { fieldType: { type: "null" }, optional: false },
        numberField: { fieldType: { type: "number" }, optional: false },
        float64Field: { fieldType: { type: "number" }, optional: false },
        int64Field: { fieldType: { type: "bigint" }, optional: false },
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
    });
  });
  test("TableDefinition includes union and object types", () => {
    const table = defineTable(
      v.union(
        v.object({ a: v.array(v.number()), b: v.optional(v.string()) }),
        v.object({ c: v.any(), d: v.bytes() }),
      ),
    ).export();
    expect(table.documentType).toEqual({
      type: "union",
      value: [
        {
          type: "object",
          value: {
            a: {
              fieldType: { type: "array", value: { type: "number" } },
              optional: false,
            },
            b: { fieldType: { type: "string" }, optional: true },
          },
        },
        {
          type: "object",
          value: {
            c: { fieldType: { type: "any" }, optional: false },
            d: { fieldType: { type: "bytes" }, optional: false },
          },
        },
      ],
    });
  });
});

test("Infer", () => {
  const documentSchema = v.object({
    property: v.string(),
  });

  type Actual = Infer<typeof documentSchema>;
  type Expected = {
    property: string;
  };

  assert<Equals<Actual, Expected>>();
});

describe("defineSchema/defineTable expose table validators", () => {
  const obj = {
    ref: v.id("reference"),
    string: v.string(),
  } as const;
  const table = defineTable(obj);
  const schema = defineSchema({ table });

  test("defineTable", () => {
    const actual = table.validator;
    const expected = v.object(obj);
    expect(actual).toEqual(expected);
    assert<Equals<typeof actual, typeof expected>>();
  });

  test("defineSchema", () => {
    const actual = schema.tables.table.validator;
    const expected = v.object(obj);
    expect(actual).toEqual(expected);
    assert<Equals<typeof actual, typeof expected>>();
  });

  test("system tables are not present", () => {
    expect(table.validator).not.toHaveProperty("_id");
    expect(table.validator).not.toHaveProperty("_creationTime");
    expect(schema.tables.table.validator).not.toHaveProperty("_id");
    expect(schema.tables.table.validator).not.toHaveProperty("_creationTime");
  });
});

test("defineTable fails if it can't export the validator", () => {
  const table = defineTable(
    // @ts-expect-error
    { ...v.object({}) }, // This will clone `isConvexValidator` but not the `json` getter used by `export`
  );
  expect(() => table.export()).toThrow(
    "Invalid validator: please make sure that the parameter of `defineTable` is valid (see https://docs.convex.dev/database/schemas)",
  );
});

describe("FlowFields, ComputedFields, and FlowFilters", () => {
  test("flowField chaining works and export includes definitions", () => {
    const table = defineTable({
      name: v.string(),
    })
      .flowField("orderCount", {
        returns: v.float64(),
        type: "count",
        source: "orders",
        key: "customerId",
      })
      .flowField("totalSpent", {
        returns: v.float64(),
        type: "sum",
        source: "orders",
        key: "customerId",
        field: "amount",
        filter: { status: "completed" },
      });

    const exported = table.export();
    expect(exported.flowFields).toEqual([
      {
        fieldName: "orderCount",
        returns: { type: "number" },
        aggregation: "count",
        source: "orders",
        key: "customerId",
        field: undefined,
        filter: undefined,
      },
      {
        fieldName: "totalSpent",
        returns: { type: "number" },
        aggregation: "sum",
        source: "orders",
        key: "customerId",
        field: "amount",
        filter: { status: "completed" },
      },
    ]);
  });

  test("computed chaining works and export includes definitions", () => {
    const table = defineTable({
      name: v.string(),
    }).computed("displayName", {
      returns: v.string(),
      expr: { $concat: ["$name", " (user)"] },
    });

    const exported = table.export();
    expect(exported.computedFields).toEqual([
      {
        fieldName: "displayName",
        returns: { type: "string" },
        expr: { $concat: ["$name", " (user)"] },
      },
    ]);
  });

  test("flowFilter chaining works and export includes definitions", () => {
    const table = defineTable({
      name: v.string(),
    }).flowFilter("dateFilter", {
      type: v.object({ from: v.float64(), to: v.float64() }),
    });

    const exported = table.export();
    expect(exported.flowFilters).toEqual([
      {
        fieldName: "dateFilter",
        filterType: {
          type: "object",
          value: {
            from: { fieldType: { type: "number" }, optional: false },
            to: { fieldType: { type: "number" }, optional: false },
          },
        },
      },
    ]);
  });

  test("mixed chaining: flowFilter, flowField, computed", () => {
    const table = defineTable({
      name: v.string(),
    })
      .flowFilter("dateFilter", {
        type: v.object({ from: v.float64(), to: v.float64() }),
      })
      .flowField("orderCount", {
        returns: v.float64(),
        type: "count",
        source: "orders",
        key: "customerId",
      })
      .computed("tier", {
        returns: v.string(),
        expr: {
          $cond: { $gt: ["$orderCount", 10] },
          $then: "VIP",
          $else: "STANDARD",
        },
      });

    const exported = table.export();
    expect(exported.flowFilters).toHaveLength(1);
    expect(exported.flowFields).toHaveLength(1);
    expect(exported.computedFields).toHaveLength(1);
  });

  test("chaining with indexes still works", () => {
    const table = defineTable({
      name: v.string(),
    })
      .index("by_name", ["name"])
      .flowField("orderCount", {
        returns: v.float64(),
        type: "count",
        source: "orders",
        key: "customerId",
      });

    const exported = table.export();
    expect(exported.indexes).toEqual([
      { indexDescriptor: "by_name", fields: ["name"] },
    ]);
    expect(exported.flowFields).toHaveLength(1);
  });

  test("empty tables have empty flow/computed/filter arrays", () => {
    const table = defineTable({
      name: v.string(),
    });

    const exported = table.export();
    expect(exported.flowFields).toEqual([]);
    expect(exported.computedFields).toEqual([]);
    expect(exported.flowFilters).toEqual([]);
  });

  test("SchemaDefinition.export() includes flow/computed/filter definitions", () => {
    const schema = defineSchema({
      customers: defineTable({
        name: v.string(),
      })
        .flowField("orderCount", {
          returns: v.float64(),
          type: "count",
          source: "orders",
          key: "customerId",
        })
        .computed("tier", {
          returns: v.string(),
          expr: {
            $cond: { $gt: ["$orderCount", 10] },
            $then: "VIP",
            $else: "STANDARD",
          },
        }),
      orders: defineTable({
        customerId: v.id("customers"),
        amount: v.float64(),
      }),
    });

    const exported = JSON.parse(schema.export());
    const customersTable = exported.tables.find(
      (t: { tableName: string }) => t.tableName === "customers",
    );
    expect(customersTable.flowFields).toHaveLength(1);
    expect(customersTable.flowFields[0].fieldName).toBe("orderCount");
    expect(customersTable.computedFields).toHaveLength(1);
    expect(customersTable.computedFields[0].fieldName).toBe("tier");
    expect(customersTable.flowFilters).toEqual([]);

    const ordersTable = exported.tables.find(
      (t: { tableName: string }) => t.tableName === "orders",
    );
    expect(ordersTable.flowFields).toEqual([]);
    expect(ordersTable.computedFields).toEqual([]);
    expect(ordersTable.flowFilters).toEqual([]);
  });

  test("type-level: document includes FlowField and ComputedField types", () => {
    const schema = defineSchema({
      customers: defineTable({
        name: v.string(),
      })
        .flowField("orderCount", {
          returns: v.float64(),
          type: "count",
          source: "orders",
          key: "customerId",
        })
        .computed("tier", {
          returns: v.string(),
          expr: {
            $cond: { $gt: ["$orderCount", 10] },
            $then: "VIP",
            $else: "STANDARD",
          },
        }),
      orders: defineTable({
        customerId: v.id("customers"),
      }),
    });

    type DataModel = DataModelFromSchemaDefinition<typeof schema>;
    type CustomerDoc = DataModel["customers"]["document"];

    // The document type should include stored fields, system fields,
    // FlowFields, and ComputedFields.
    type ExpectedDocument = {
      _id: GenericId<"customers">;
      _creationTime: number;
      name: string;
      orderCount: number;
      tier: string;
    };

    assert<Equals<CustomerDoc, ExpectedDocument>>();
  });

  test("type-level: FlowFilter types are tracked but not in document", () => {
    const schema = defineSchema({
      customers: defineTable({
        name: v.string(),
      }).flowFilter("dateFilter", {
        type: v.object({ from: v.float64(), to: v.float64() }),
      }),
    });

    type DataModel = DataModelFromSchemaDefinition<typeof schema>;
    type CustomerDoc = DataModel["customers"]["document"];

    // FlowFilters should NOT appear in the document type.
    type ExpectedDocument = {
      _id: GenericId<"customers">;
      _creationTime: number;
      name: string;
    };

    assert<Equals<CustomerDoc, ExpectedDocument>>();
  });

  test("type-level: full schema with all features", () => {
    const schema = defineSchema({
      customers: defineTable({
        name: v.string(),
      })
        .index("by_name", ["name"])
        .flowFilter("dateFilter", {
          type: v.object({ from: v.float64(), to: v.float64() }),
        })
        .flowField("orderCount", {
          returns: v.float64(),
          type: "count",
          source: "orders",
          key: "customerId",
        })
        .flowField("totalSpent", {
          returns: v.float64(),
          type: "sum",
          source: "orders",
          key: "customerId",
          field: "amount",
        })
        .computed("tier", {
          returns: v.string(),
          expr: {
            $cond: { $gt: ["$totalSpent", 1000] },
            $then: "VIP",
            $else: "STANDARD",
          },
        }),
      orders: defineTable({
        customerId: v.id("customers"),
        amount: v.float64(),
      }),
    });

    type DataModel = DataModelFromSchemaDefinition<typeof schema>;

    type ExpectedDocument = {
      _id: GenericId<"customers">;
      _creationTime: number;
      name: string;
      orderCount: number;
      totalSpent: number;
      tier: string;
    };

    type ExpectedIndexes = {
      by_name: ["name", "_creationTime"];
      by_id: ["_id"];
      by_creation_time: ["_creationTime"];
    };

    type ExpectedDataModel = {
      customers: {
        document: ExpectedDocument;
        fieldPaths: "_id" | "_creationTime" | "name";
        indexes: ExpectedIndexes;
        searchIndexes: {};
        vectorIndexes: {};
        computedFields: "orderCount" | "totalSpent" | "tier";
      };
      orders: {
        document: {
          _id: GenericId<"orders">;
          _creationTime: number;
          customerId: GenericId<"customers">;
          amount: number;
        };
        fieldPaths: "_id" | "_creationTime" | "customerId" | "amount";
        indexes: {
          by_id: ["_id"];
          by_creation_time: ["_creationTime"];
        };
        searchIndexes: {};
        vectorIndexes: {};
        computedFields: never;
      };
    };

    assert<Equals<DataModel, ExpectedDataModel>>();
  });

  test("type-level: FlowFields and ComputedFields excluded from WritableFields", () => {
    const schema = defineSchema({
      customers: defineTable({
        name: v.string(),
        email: v.string(),
      })
        .flowField("orderCount", {
          returns: v.float64(),
          type: "count",
          source: "orders",
          key: "customerId",
        })
        .computed("label", {
          returns: v.string(),
          expr: { $concat: ["$name", " (", "$email", ")"] },
        }),
      orders: defineTable({
        customerId: v.id("customers"),
      }),
    });

    type DataModel = DataModelFromSchemaDefinition<typeof schema>;

    // WritableFields should exclude flow/computed fields and system fields.
    type Writable = import("./system_fields.js").WritableFields<
      DataModel["customers"]
    >;

    type ExpectedWritable = {
      name: string;
      email: string;
    };

    assert<Equals<Writable, ExpectedWritable>>();
  });

  test("type-level: ReplaceValue excludes FlowFields/ComputedFields, makes system fields optional", () => {
    const schema = defineSchema({
      customers: defineTable({
        name: v.string(),
      })
        .flowField("orderCount", {
          returns: v.float64(),
          type: "count",
          source: "orders",
          key: "customerId",
        }),
      orders: defineTable({
        customerId: v.id("customers"),
      }),
    });

    type DataModel = DataModelFromSchemaDefinition<typeof schema>;

    type Replace = import("./system_fields.js").ReplaceValue<
      DataModel["customers"]
    >;

    type ExpectedReplace = {
      name: string;
      _id?: GenericId<"customers">;
      _creationTime?: number;
    };

    assert<Equals<Replace, ExpectedReplace>>();
  });

  test("type-level: FlowExpr provides autocomplete for field references", () => {
    // FlowExpr<"name" | "totalSpent"> should accept $-prefixed field names
    const fieldRef: FlowExpr<"name" | "totalSpent"> = "$name";
    const fieldRef2: FlowExpr<"name" | "totalSpent"> = "$totalSpent";
    const literal: FlowExpr<"name" | "totalSpent"> = "VIP";
    const num: FlowExpr<"name" | "totalSpent"> = 42;
    const bool: FlowExpr<"name" | "totalSpent"> = true;
    const nil: FlowExpr<"name" | "totalSpent"> = null;
    void fieldRef;
    void fieldRef2;
    void literal;
    void num;
    void bool;
    void nil;

    // Nested expressions maintain field name generics
    const cond: FlowExpr<"name" | "totalSpent"> = {
      $cond: { $gt: ["$totalSpent", 1000] },
      $then: "VIP",
      $else: "STANDARD",
    };
    void cond;

    const concat: FlowExpr<"name" | "totalSpent"> = {
      $concat: ["$name", " (", "$totalSpent", ")"],
    };
    void concat;
  });

  test("type-level: FlowFieldFilter provides autocomplete for $field refs", () => {
    // FlowFieldFilter<"dateFilter"> should accept { $field: "dateFilter" }
    const filter: FlowFieldFilter<"dateFilter" | "dimFilter"> = {
      postingDate: { $field: "dateFilter" },
      dimension: { $field: "dimFilter" },
      status: "completed",
      count: 5,
      active: true,
    };
    void filter;
  });

  test("type-level: tables without flow fields have computedFields: never", () => {
    const schema = defineSchema({
      simple: defineTable({
        value: v.string(),
      }),
    });

    type DataModel = DataModelFromSchemaDefinition<typeof schema>;

    // computedFields should be never for tables without flow/computed fields.
    type CF = DataModel["simple"]["computedFields"];
    assert<Equals<CF, never>>();

    // WritableFields should include all user fields.
    type Writable = import("./system_fields.js").WritableFields<
      DataModel["simple"]
    >;
    type ExpectedWritable = {
      value: string;
    };
    assert<Equals<Writable, ExpectedWritable>>();
  });
});
