import { GenericDocument, Index } from "convex/server";
import {
  Filter,
  FilterExpressionSchema,
  SchemaJson,
  ValidFilterByBuiltInOrOr,
  ValidFilterByType,
  applyTypeFilters,
  findErrorsInFilters,
  isFilterValidationError,
  isValidFilter,
  parseAndFilterToSingleTable,
  partitionFiltersByOperator,
} from "./filters";
import { partitionFiltersByIndexes } from "./filters";

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
  });

  describe("isValidFilter", () => {
    it("should return true for valid filter", () => {
      expect(
        isValidFilter({ field: "name", op: "eq", value: "foo" }),
      ).toBeTruthy();
    });

    it("should return false for invalid filter", () => {
      expect(isValidFilter({ op: "eq" })).toBeFalsy();
      expect(isValidFilter({ op: "eq", field: "abc" })).toBeFalsy();
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

  describe("partitionFiltersByIndexes", () => {
    type TestCase = {
      name: string;
      filters: ValidFilterByBuiltInOrOr[];
      indexes: Index[];
      expected: {
        selectedIndex?: string;
        indexFilter: ValidFilterByBuiltInOrOr[];
        nonIndexFilter: ValidFilterByBuiltInOrOr[];
      };
    };
    const testCases: TestCase[] = [
      {
        name: "should partition filters correctly",
        filters: [
          { op: "eq", field: "field1", id: "", value: "" },
          { op: "eq", field: "field2", id: "", value: "" },
          { op: "eq", field: "field3", id: "", value: "" },
        ],
        indexes: [
          { fields: ["field1", "field2"], indexDescriptor: "index1" },
          { fields: ["field3"], indexDescriptor: "index2" },
        ],
        expected: {
          selectedIndex: "index1",
          indexFilter: [
            { op: "eq", field: "field1", id: "", value: "" },
            { op: "eq", field: "field2", id: "", value: "" },
          ],
          nonIndexFilter: [{ op: "eq", field: "field3", id: "", value: "" }],
        },
      },
      {
        name: "should handle empty filters array",
        filters: [],
        indexes: [{ fields: ["field1", "field2"], indexDescriptor: "index1" }],
        expected: {
          selectedIndex: undefined,
          indexFilter: [],
          nonIndexFilter: [],
        },
      },
      {
        name: "should handle empty indexes array",
        filters: [
          { op: "eq", field: "field1", id: "", value: "" },
          { op: "eq", field: "field2", id: "", value: "" },
        ],
        indexes: [],
        expected: {
          selectedIndex: undefined,
          indexFilter: [],
          nonIndexFilter: [
            { op: "eq", field: "field1", id: "", value: "" },
            { op: "eq", field: "field2", id: "", value: "" },
          ],
        },
      },
      {
        name: "should handle multiple matching indexes, selecting the match with more fields",
        filters: [
          { op: "eq", field: "field1", id: "", value: "" },
          { op: "eq", field: "field2", id: "", value: "" },
        ],
        indexes: [
          { fields: ["field1"], indexDescriptor: "index1" },
          { fields: ["field1", "field2"], indexDescriptor: "index2" },
        ],
        expected: {
          selectedIndex: "index2",
          indexFilter: [
            { op: "eq", field: "field1", id: "", value: "" },
            { op: "eq", field: "field2", id: "", value: "" },
          ],
          nonIndexFilter: [],
        },
      },
      {
        name: "should handle no matching fields in indexes",
        filters: [
          { op: "eq", field: "field1", id: "", value: "" },
          { op: "eq", field: "field2", id: "", value: "" },
        ],
        indexes: [{ fields: ["field3", "field4"], indexDescriptor: "index1" }],
        expected: {
          selectedIndex: undefined,
          indexFilter: [],
          nonIndexFilter: [
            { op: "eq", field: "field1", id: "", value: "" },
            { op: "eq", field: "field2", id: "", value: "" },
          ],
        },
      },
      {
        name: "should handle partial match with an index",
        filters: [
          { op: "eq", field: "field1", id: "", value: "" },
          { op: "eq", field: "field2", id: "", value: "" },
        ],
        indexes: [{ fields: ["field1", "field3"], indexDescriptor: "index1" }],
        expected: {
          selectedIndex: "index1",
          indexFilter: [{ op: "eq", field: "field1", id: "", value: "" }],
          nonIndexFilter: [{ op: "eq", field: "field2", id: "", value: "" }],
        },
      },
      {
        name: "should handle filters with duplicate fields",
        filters: [
          { op: "eq", field: "field1", id: "", value: "" },
          { op: "eq", field: "field1", id: "", value: "" },
        ],
        indexes: [{ fields: ["field1"], indexDescriptor: "index1" }],
        expected: {
          selectedIndex: "index1",
          indexFilter: [{ op: "eq", field: "field1", id: "", value: "" }],
          nonIndexFilter: [{ op: "eq", field: "field1", id: "", value: "" }],
        },
      },
      {
        name: "should return all filters as nonIndexFilter when no index matches",
        filters: [
          { op: "eq", field: "field1", id: "", value: "" },
          { op: "eq", field: "field2", id: "", value: "" },
        ],
        indexes: [{ fields: ["field3", "field4"], indexDescriptor: "index1" }],
        expected: {
          selectedIndex: undefined,
          indexFilter: [],
          nonIndexFilter: [
            { op: "eq", field: "field1", id: "", value: "" },
            { op: "eq", field: "field2", id: "", value: "" },
          ],
        },
      },
      {
        name: "should return the correct filter if the index is a subset of the filters",
        filters: [
          { op: "eq", field: "field1", id: "", value: "" },
          { op: "eq", field: "field2", id: "", value: "" },
        ],
        indexes: [
          { fields: ["field1", "field2", "field3"], indexDescriptor: "index2" },
          { fields: ["field1", "field3"], indexDescriptor: "index1" },
        ],
        expected: {
          selectedIndex: "index2",
          indexFilter: [
            { op: "eq", field: "field1", id: "", value: "" },
            { op: "eq", field: "field2", id: "", value: "" },
          ],
          nonIndexFilter: [],
        },
      },
      {
        name: "should return the correct filter if the index is a subset of the filters and the wrong index is second",
        filters: [
          { op: "eq", field: "field1", id: "", value: "" },
          { op: "eq", field: "field2", id: "", value: "" },
        ],
        indexes: [
          { fields: ["field1", "field3"], indexDescriptor: "index1" },
          { fields: ["field1", "field2", "field3"], indexDescriptor: "index2" },
        ],
        expected: {
          selectedIndex: "index2",
          indexFilter: [
            { op: "eq", field: "field1", id: "", value: "" },
            { op: "eq", field: "field2", id: "", value: "" },
          ],
          nonIndexFilter: [],
        },
      },
    ];

    test.each<TestCase>(testCases)(
      "$name",
      ({ filters, indexes, expected }) => {
        const [selectedIndex, indexFilter, nonIndexFilter] =
          partitionFiltersByIndexes(filters, indexes);

        expect(selectedIndex).toEqual(expected.selectedIndex);
        expect(indexFilter).toEqual(expected.indexFilter);
        expect(nonIndexFilter).toEqual(expected.nonIndexFilter);
      },
    );
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
});
