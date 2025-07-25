import { describe, test, expect, it } from "vitest";
import { GenericDocument } from "convex/server";
import {
  Filter,
  FilterByIndex,
  FilterByIndexRange,
  FilterExpressionSchema,
  SchemaJson,
  ValidFilterByType,
  applyIndexFilters,
  applyTypeFilters,
  findErrorsInFilters,
  findIndexByName,
  getAvailableIndexes,
  getDefaultIndex,
  isFilterValidationError,
  isValidFilter,
  parseAndFilterToSingleTable,
  partitionFiltersByOperator,
  validateIndexFilter,
} from "./filters";

const samplePage: GenericDocument[] = [
  { variableType: "stringValue" },
  { variableType: "stringValue", name: "blah" },
  { variableType: false },
  { variableType: 34 },
  { variableType: BigInt(34) },
  { variableType: null },
  { variableType: "z43zp6c3e75gkmz1kfwj6mbbx5sw281h" },
  { variableType: ["a", 2, true] },
  { variableType: { a: 1, b: null, c: true } },
  { variableType: new ArrayBuffer(8) },
  // variableType is undefined
  { v: 1 },
];

describe("filters", () => {
  describe("paritionFiltersByOperator", () => {
    it("should partition filters by operator", () => {
      expect(
        partitionFiltersByOperator([
          { field: "foo", op: "type", value: "string" },
          { field: "foo", op: "eq", value: "bar" },
          { field: "foo", op: "notype", value: "string" },
          { field: "foo", op: "neq", value: "bar" },
        ]),
      ).toStrictEqual([
        [
          { field: "foo", op: "eq", value: "bar" },
          { field: "foo", op: "neq", value: "bar" },
        ],
        [
          { field: "foo", op: "type", value: "string" },
          { field: "foo", op: "notype", value: "string" },
        ],
      ]);
    });

    it("should partition filters with enabled field", () => {
      expect(
        partitionFiltersByOperator([
          { field: "foo", op: "type", value: "string", enabled: true },
          { field: "foo", op: "eq", value: "bar", enabled: false },
          { field: "foo", op: "notype", value: "string", enabled: true },
          { field: "foo", op: "neq", value: "bar", enabled: false },
        ]),
      ).toStrictEqual([
        [
          { field: "foo", op: "eq", value: "bar", enabled: false },
          { field: "foo", op: "neq", value: "bar", enabled: false },
        ],
        [
          { field: "foo", op: "type", value: "string", enabled: true },
          { field: "foo", op: "notype", value: "string", enabled: true },
        ],
      ]);
    });

    it("should return empty lists", () => {
      expect(partitionFiltersByOperator([])).toStrictEqual([[], []]);
    });

    it("should return empty type list", () => {
      expect(
        partitionFiltersByOperator([{ field: "foo", op: "eq", value: "bar" }]),
      ).toStrictEqual([[{ field: "foo", op: "eq", value: "bar" }], []]);
    });

    it("should return empty builtin list", () => {
      expect(
        partitionFiltersByOperator([
          { field: "foo", op: "type", value: "string" },
        ]),
      ).toStrictEqual([[], [{ field: "foo", op: "type", value: "string" }]]);
    });
  });

  describe("applyTypeFilters", () => {
    test.each<{
      name: string;
      page: GenericDocument[];
      filters: (
        | Omit<ValidFilterByType, "id">
        | (Omit<ValidFilterByType, "id" | "enabled"> & { enabled?: boolean })
      )[];
      expected: GenericDocument[];
    }>([
      {
        name: "type: string",
        page: samplePage,
        filters: [{ field: "variableType", op: "type", value: "string" }],
        expected: [samplePage[0], samplePage[1], samplePage[6]],
      },
      {
        name: "type: boolean",
        page: samplePage,
        filters: [{ field: "variableType", op: "type", value: "boolean" }],
        expected: [samplePage[2]],
      },
      {
        name: "type: number",
        page: samplePage,
        filters: [{ field: "variableType", op: "type", value: "number" }],
        expected: [samplePage[3]],
      },
      {
        name: "type: bigint",
        page: samplePage,
        filters: [{ field: "variableType", op: "type", value: "bigint" }],
        expected: [samplePage[4]],
      },
      {
        name: "type: null",
        page: samplePage,
        filters: [{ field: "variableType", op: "type", value: "null" }],
        expected: [samplePage[5]],
      },
      {
        name: "type: id",
        page: samplePage,
        filters: [{ field: "variableType", op: "type", value: "id" }],
        expected: [samplePage[6]],
      },
      {
        name: "type: array",
        page: samplePage,
        filters: [{ field: "variableType", op: "type", value: "array" }],
        expected: [samplePage[7]],
      },
      {
        name: "type: object",
        page: samplePage,
        filters: [{ field: "variableType", op: "type", value: "object" }],
        expected: [samplePage[8]],
      },
      {
        name: "type: bytes",
        page: samplePage,
        filters: [{ field: "variableType", op: "type", value: "bytes" }],
        expected: [samplePage[9]],
      },
      {
        name: "type: unset",
        page: samplePage,
        filters: [{ field: "variableType", op: "type", value: "unset" }],
        expected: [samplePage[10]],
      },
      {
        name: "no matches",
        page: samplePage,
        filters: [{ field: "name", op: "type", value: "number" }],
        expected: [],
      },
      {
        name: "multiple filters",
        page: samplePage,
        filters: [
          { field: "variableType", op: "type", value: "string" },
          { field: "name", op: "type", value: "string" },
        ],
        expected: [samplePage[1]],
      },
      {
        name: "respects enabled=false",
        page: samplePage,
        filters: [
          {
            field: "variableType",
            op: "type",
            value: "string",
            enabled: false,
          },
        ],
        expected: samplePage,
      },
      {
        name: "respects enabled=true",
        page: samplePage,
        filters: [
          { field: "variableType", op: "type", value: "string", enabled: true },
        ],
        expected: [samplePage[0], samplePage[1], samplePage[6]],
      },
      {
        name: "mixed enabled filters",
        page: samplePage,
        filters: [
          { field: "variableType", op: "type", value: "string", enabled: true },
          { field: "name", op: "type", value: "string", enabled: false },
        ],
        expected: [samplePage[0], samplePage[1], samplePage[6]],
      },
      {
        name: "respects missing enabled field (backward compatibility)",
        page: samplePage,
        filters: [
          { field: "variableType", op: "type", value: "string" }, // enabled field is missing
        ],
        expected: [samplePage[0], samplePage[1], samplePage[6]],
      },
    ])(`filterPage $name`, ({ page, filters, expected }) => {
      const filteredPage = applyTypeFilters(
        page,
        filters as ValidFilterByType[],
      );
      expect(filteredPage).toEqual(expected);
    });
  });

  describe("findErrorsInFilters", () => {
    it("should not return a validation error for valid values", async () => {
      const filters: Filter[] = [
        {
          field: "name",
          op: "eq",
          value: "sarah",
        },
      ];
      const errors = await findErrorsInFilters({ clauses: filters });
      expect(errors).toHaveLength(0);
    });

    it("should return a validation errors for invalid values", async () => {
      const filters: Filter[] = [
        { field: "name", op: "eq", value: { $foo: "bar" } },
        {
          field: "name",
          op: "eq",
          value: { $bar: "foo" },
        },
      ];
      const errors = await findErrorsInFilters({ clauses: filters });
      expect(errors).toHaveLength(2);
      expect(errors[0]).toStrictEqual({
        error: 'Invalid value: {"$foo":"bar"}.',
        filter: 0,
      });
      expect(errors[1]).toStrictEqual({
        error: 'Invalid value: {"$bar":"foo"}.',
        filter: 1,
      });
    });
  });

  describe("FilterExpressionSchema", () => {
    it("should throw an error if the filter expression is not valid", () => {
      const invalidFilterExpression = {
        clauses: [
          {
            field: "name",
            op: "type",
            // This is not allowed
            value: "invalid",
          },
        ],
      };
      expect(() =>
        FilterExpressionSchema.parse(invalidFilterExpression),
      ).toThrowError();
    });

    it("should validate a valid filter expression", () => {
      const validFilterExpression = {
        op: "and",
        clauses: [
          {
            field: "name",
            op: "eq",
            value: "foo",
          },
          {
            field: "name",
            op: "type",
            value: "bytes",
          },
        ],
      };
      expect(FilterExpressionSchema.parse(validFilterExpression)).toEqual(
        validFilterExpression,
      );
    });

    it("should validate a valid filter expression with order", () => {
      const validFilterExpressionWithOrder = {
        op: "and",
        clauses: [
          {
            field: "name",
            op: "eq",
            value: "foo",
          },
        ],
        order: "asc",
      };
      expect(
        FilterExpressionSchema.parse(validFilterExpressionWithOrder),
      ).toEqual(validFilterExpressionWithOrder);
    });
  });

  describe("isValidFilter", () => {
    it("should return true for valid filter", () => {
      expect(
        isValidFilter({ field: "name", op: "eq", value: "foo" }),
      ).toBeTruthy();
      expect(isValidFilter({ op: "eq", field: "abc" })).toBeTruthy();
    });

    it("should return false for invalid filter", () => {
      expect(isValidFilter({ op: "eq" })).toBeFalsy();
      expect(isValidFilter({ op: "eq", value: "abc" })).toBeFalsy();
    });
  });

  describe("isFilterValidationError", () => {
    it("should return true for filter validation error", () => {
      // Is an error
      expect(
        isFilterValidationError({
          error: "Invalid filter",
          filter: 0,
        }),
      ).toBeTruthy();
    });

    it("should return false for invalid filter validation error", () => {
      // Is a document
      expect(isFilterValidationError({ _id: "abcdef" })).toBeFalsy();
    });
  });

  describe("applyIndexFilters", () => {
    // Mock query builder to test the function
    class MockQueryBuilder {
      operations: { op: string; field: string; value: any }[] = [];

      eq(field: string, value: any) {
        this.operations.push({ op: "eq", field, value });
        return this;
      }

      lt(field: string, value: any) {
        this.operations.push({ op: "lt", field, value });
        return this;
      }

      lte(field: string, value: any) {
        this.operations.push({ op: "lte", field, value });
        return this;
      }

      gt(field: string, value: any) {
        this.operations.push({ op: "gt", field, value });
        return this;
      }

      gte(field: string, value: any) {
        this.operations.push({ op: "gte", field, value });
        return this;
      }
    }

    it("should apply equality filters correctly", () => {
      const mockQuery = new MockQueryBuilder();
      const indexFilters: FilterByIndex[] = [
        { type: "indexEq", enabled: true, value: "value1" },
        { type: "indexEq", enabled: true, value: 42 },
      ];
      const selectedIndex = {
        fields: ["field1", "field2"],
        indexDescriptor: "testIndex",
      };

      const result = applyIndexFilters(mockQuery, indexFilters, selectedIndex);

      expect(result.operations).toEqual([
        { op: "eq", field: "field1", value: "value1" },
        { op: "eq", field: "field2", value: 42 },
      ]);
    });

    it("should handle empty filters array", () => {
      const mockQuery = new MockQueryBuilder();
      const indexFilters: FilterByIndex[] = [];
      const selectedIndex = {
        fields: ["field1", "field2"],
        indexDescriptor: "testIndex",
      };

      const result = applyIndexFilters(mockQuery, indexFilters, selectedIndex);

      expect(result.operations).toEqual([]);
    });

    it("should apply range filter correctly", () => {
      const mockQuery = new MockQueryBuilder();
      const indexFilters: [...FilterByIndex[], FilterByIndexRange] = [
        { type: "indexEq", enabled: true, value: "value1" },
        {
          type: "indexRange",
          enabled: true,
          lowerOp: "gt",
          lowerValue: 10,
          upperOp: "lt",
          upperValue: 50,
        },
      ];
      const selectedIndex = {
        fields: ["field1", "field2"],
        indexDescriptor: "testIndex",
      };

      const result = applyIndexFilters(mockQuery, indexFilters, selectedIndex);

      expect(result.operations).toEqual([
        { op: "eq", field: "field1", value: "value1" },
        { op: "gt", field: "field2", value: 10 },
        { op: "lt", field: "field2", value: 50 },
      ]);
    });

    it("should throw error if range filter is not the last filter", () => {
      const mockQuery = new MockQueryBuilder();
      // This is intentionally wrong to test the error case
      const indexFilters = [
        {
          enabled: true,
          lowerOp: "gt",
          lowerValue: 10,
          upperOp: "lt",
          upperValue: 50,
        } as FilterByIndexRange,
        { enabled: true, value: "value1" } as FilterByIndex,
      ];
      const selectedIndex = {
        fields: ["field1", "field2"],
        indexDescriptor: "testIndex",
      };

      expect(() => {
        // @ts-expect-error - We're intentionally passing an invalid structure to test error handling
        applyIndexFilters(mockQuery, indexFilters, selectedIndex);
      }).toThrow("Index range not supported");
    });

    it("should handle partial range filter with only lowerOp", () => {
      const mockQuery = new MockQueryBuilder();
      const indexFilters: [...FilterByIndex[], FilterByIndexRange] = [
        { type: "indexEq", enabled: true, value: "value1" },
        {
          type: "indexRange",
          enabled: true,
          lowerOp: "gt",
          lowerValue: 10,
        },
      ];
      const selectedIndex = {
        fields: ["field1", "field2"],
        indexDescriptor: "testIndex",
      };

      const result = applyIndexFilters(mockQuery, indexFilters, selectedIndex);

      expect(result.operations).toEqual([
        { op: "eq", field: "field1", value: "value1" },
        { op: "gt", field: "field2", value: 10 },
      ]);
    });

    it("should handle partial range filter with only upperOp", () => {
      const mockQuery = new MockQueryBuilder();
      const indexFilters: [...FilterByIndex[], FilterByIndexRange] = [
        { type: "indexEq", enabled: true, value: "value1" },
        {
          type: "indexRange",
          enabled: true,
          upperOp: "lt",
          upperValue: 50,
        },
      ];
      const selectedIndex = {
        fields: ["field1", "field2"],
        indexDescriptor: "testIndex",
      };

      const result = applyIndexFilters(mockQuery, indexFilters, selectedIndex);

      expect(result.operations).toEqual([
        { op: "eq", field: "field1", value: "value1" },
        { op: "lt", field: "field2", value: 50 },
      ]);
    });

    it("should throw error if selectedIndex is undefined", () => {
      const mockQuery = new MockQueryBuilder();
      const indexFilters: FilterByIndex[] = [
        { type: "indexEq", enabled: true, value: "value1" },
      ];
      const selectedIndex = undefined;

      expect(() => {
        // @ts-expect-error - We're intentionally passing undefined to test error handling
        applyIndexFilters(mockQuery, indexFilters, selectedIndex);
      }).toThrow("Index is undefined");
    });
  });

  describe("validateIndexFilter", () => {
    it("should return undefined for valid index filter", () => {
      const indexName = "testIndex";
      const indexClauses: FilterByIndex[] = [
        { type: "indexEq", enabled: true, value: "value1" },
        { type: "indexEq", enabled: true, value: 42 },
      ];
      const selectedIndex = {
        fields: ["field1", "field2"],
        indexDescriptor: "testIndex",
      };

      const result = validateIndexFilter(
        indexName,
        indexClauses,
        selectedIndex,
      );
      expect(result).toBeUndefined();
    });

    it("should return error when index does not exist", () => {
      const indexName = "testIndex";
      const indexClauses: FilterByIndex[] = [
        { type: "indexEq", enabled: true, value: "value1" },
      ];
      const selectedIndex = undefined;

      const result = validateIndexFilter(
        indexName,
        indexClauses,
        selectedIndex,
      );
      expect(result).toEqual({
        filter: -1,
        error: "Index testIndex does not exist.",
      });
    });

    it("should return error when clauses exceed index fields", () => {
      const indexName = "testIndex";
      const indexClauses: FilterByIndex[] = [
        { type: "indexEq", enabled: true, value: "value1" },
        { type: "indexEq", enabled: true, value: 42 },
        { type: "indexEq", enabled: true, value: true },
      ];
      const selectedIndex = {
        fields: ["field1", "field2"],
        indexDescriptor: "testIndex",
      };

      const result = validateIndexFilter(
        indexName,
        indexClauses,
        selectedIndex,
      );
      expect(result).toEqual({
        filter: -1,
        error: "Index testIndex has 2 fields, but the query has 3 clauses.",
      });
    });

    it("should return error when enabled clauses are not contiguous", () => {
      const indexName = "testIndex";
      const indexClauses: FilterByIndex[] = [
        { type: "indexEq", enabled: true, value: "value1" },
        { type: "indexEq", enabled: false, value: 42 },
        { type: "indexEq", enabled: true, value: true },
      ];
      const selectedIndex = {
        fields: ["field1", "field2", "field3"],
        indexDescriptor: "testIndex",
      };

      const result = validateIndexFilter(
        indexName,
        indexClauses,
        selectedIndex,
      );
      expect(result).toEqual({
        filter: -1,
        error:
          "Invalid index filter selection - found an enabled clause after an disabled clause.",
      });
    });

    it("should return error when range filter is not the last clause", () => {
      const indexName = "testIndex";
      const indexClauses = [
        { type: "indexEq", enabled: true, value: "value1" } as FilterByIndex,
        {
          type: "indexRange",
          enabled: true,
          lowerOp: "gt",
          lowerValue: 10,
        } as FilterByIndexRange,
        { type: "indexEq", enabled: true, value: true } as FilterByIndex,
        { type: "indexEq", enabled: false, value: "disabled" } as FilterByIndex,
      ];
      const selectedIndex = {
        fields: ["field1", "field2", "field3", "field4"],
        indexDescriptor: "testIndex",
      };

      const result = validateIndexFilter(
        indexName,
        indexClauses,
        selectedIndex,
      );
      expect(result).toEqual({
        filter: -1,
        error:
          "Invalid index filter selection - found a range filter after a non-range filter.",
      });
    });
  });

  describe("parseAndFilterToSingleTable", () => {
    type TestCase = {
      name: string;
      tableName: string;
      schema: string | null;
      expected: SchemaJson | undefined;
    };

    const testCases: TestCase[] = [
      {
        name: "should return undefined when schema is null",
        tableName: "table1",
        schema: null,
        expected: undefined,
      },
      {
        name: "should return filtered schema when table name matches",
        tableName: "table1",
        schema: JSON.stringify({
          tables: [
            {
              tableName: "table1",
              indexes: [],
              searchIndexes: [],
              documentType: { type: "any" },
            },
            {
              tableName: "table2",
              indexes: [],
              searchIndexes: [],
              documentType: { type: "any" },
            },
          ],
          schemaValidation: true,
        }),
        expected: {
          tables: [
            {
              tableName: "table1",
              indexes: [],
              searchIndexes: [],
              documentType: { type: "any" },
            },
          ],
          schemaValidation: true,
        },
      },
      {
        name: "should return schema with empty tables array when table name does not match",
        tableName: "table3",
        schema: JSON.stringify({
          tables: [
            {
              tableName: "table1",
              indexes: [],
              searchIndexes: [],
              documentType: {},
            },
            {
              tableName: "table2",
              indexes: [],
              searchIndexes: [],
              documentType: {},
            },
          ],
          schemaValidation: true,
        }),
        expected: {
          tables: [],
          schemaValidation: true,
        },
      },
    ];

    test.each<TestCase>(testCases)(
      "$name",
      ({ tableName, schema, expected }) => {
        const result = parseAndFilterToSingleTable(tableName, schema);
        expect(result).toEqual(expected);
      },
    );
  });

  describe("getDefaultIndex", () => {
    it("should return the default creation time index", () => {
      const defaultIndex = getDefaultIndex();

      expect(defaultIndex).toEqual({
        indexDescriptor: "by_creation_time",
        fields: ["_creationTime"],
      });
    });
  });

  describe("getAvailableIndexes", () => {
    it("should return only default index when schema is null", () => {
      const indexes = getAvailableIndexes("testTable", null);

      expect(indexes).toEqual([
        {
          indexDescriptor: "by_creation_time",
          fields: ["_creationTime"],
        },
        {
          indexDescriptor: "by_id",
          fields: ["_id"],
        },
      ]);
    });

    it("should return table indexes plus default index", () => {
      const schemaData = {
        schema: JSON.stringify({
          tables: [
            {
              tableName: "testTable",
              indexes: [
                {
                  indexDescriptor: "by_name",
                  fields: ["name"],
                },
              ],
              searchIndexes: [],
              documentType: { type: "any" },
            },
          ],
          schemaValidation: true,
        }),
      };

      const indexes = getAvailableIndexes("testTable", schemaData);

      expect(indexes).toEqual([
        {
          indexDescriptor: "by_name",
          fields: ["name"],
        },
        {
          indexDescriptor: "by_creation_time",
          fields: ["_creationTime"],
        },
        {
          indexDescriptor: "by_id",
          fields: ["_id"],
        },
      ]);
    });

    it("should return only default index when table not found", () => {
      const schemaData = {
        schema: JSON.stringify({
          tables: [
            {
              tableName: "otherTable",
              indexes: [
                {
                  indexDescriptor: "by_name",
                  fields: ["name"],
                },
              ],
              searchIndexes: [],
              documentType: { type: "any" },
            },
          ],
          schemaValidation: true,
        }),
      };

      const indexes = getAvailableIndexes("testTable", schemaData);

      expect(indexes).toEqual([
        {
          indexDescriptor: "by_creation_time",
          fields: ["_creationTime"],
        },
        {
          indexDescriptor: "by_id",
          fields: ["_id"],
        },
      ]);
    });
  });

  describe("findIndexByName", () => {
    it("should find index by name", () => {
      const indexes = [
        {
          indexDescriptor: "by_name",
          fields: ["name"],
        },
        {
          indexDescriptor: "by_creation_time",
          fields: ["_creationTime"],
        },
      ];

      const result = findIndexByName("by_name", indexes);

      expect(result).toEqual({
        indexDescriptor: "by_name",
        fields: ["name"],
      });
    });

    it("should return undefined when index not found", () => {
      const indexes = [
        {
          indexDescriptor: "by_name",
          fields: ["name"],
        },
        {
          indexDescriptor: "by_creation_time",
          fields: ["_creationTime"],
        },
      ];

      const result = findIndexByName("by_age", indexes);

      expect(result).toBeUndefined();
    });

    it("should return undefined for empty indexes array", () => {
      const result = findIndexByName("by_name", []);

      expect(result).toBeUndefined();
    });
  });
});
