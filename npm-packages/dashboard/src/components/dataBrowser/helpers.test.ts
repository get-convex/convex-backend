import { Shape } from "shapes";
import { defaultValueForShape, sortColumns } from "./helpers";

jest.mock("api/profile", () => {});
jest.mock("api/teams", () => {});

describe("helpers", () => {
  it("sorts _id first and _creationTime last", () => {
    const returned = sortColumns(["b", "a", "_id", "_creationTime"]);
    expect(returned).toEqual(["_id", "a", "b", "_creationTime"]);
  });

  describe("defaultValueForShape", () => {
    test.each([
      {
        name: "id",
        shape: { type: "Id", tableName: "table1" },
        expected: "",
      },
      {
        name: "string",
        shape: { type: "String" },
        expected: "",
      },
      {
        name: "boolean",
        shape: { type: "Boolean" },
        expected: false,
      },
      {
        name: "float",
        shape: { type: "Float64" },
        expected: 0,
      },
      {
        name: "bigint",
        shape: { type: "Int64" },
        expected: BigInt(0),
      },
      {
        name: "array",
        shape: { type: "Array" },
        expected: [],
      },
      {
        name: "object",
        shape: {
          type: "Object",
          fields: [
            {
              fieldName: "abc",
              shape: { type: "String" },
            },
            {
              fieldName: "nested",
              shape: {
                type: "Object",
                fields: [{ fieldName: "def", shape: { type: "String" } }],
              },
            },
          ],
        },
        expected: { abc: "", nested: { def: "" } },
      },
      {
        name: "union",
        shape: {
          type: "Union",
          shapes: [{ type: "String" }, { type: "Number" }],
        },
        expected: "",
      },
      { name: "null", shape: { type: "Null" }, expected: null },
    ])(
      "returns the correct default value for a $name",
      ({ shape, expected }) => {
        const returned = defaultValueForShape(shape as Shape);
        expect(returned).toEqual(expected);
      },
    );
  });
});
