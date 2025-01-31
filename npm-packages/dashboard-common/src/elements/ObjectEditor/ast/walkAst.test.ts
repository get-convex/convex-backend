import { ValidatorJSON, Value } from "convex/values";
import { UNDEFINED_PLACEHOLDER } from "system-udfs/convex/_system/frontend/patchDocumentsFields";
import * as Base64 from "base64-js";
import { stringifyValue } from "lib/stringifyValue";
import { walkAst } from "elements/ObjectEditor/ast/walkAst";
import { SchemaValidationErrorCode } from "elements/ObjectEditor/ast/types";
import { validateConvexFieldName } from "elements/ObjectEditor/ast/ast";

const validators: Record<string, ValidatorJSON> = {
  number: { type: "number" },
  bigint: { type: "bigint" },
  boolean: { type: "boolean" },
  string: { type: "string" },
  null: { type: "null" },
  id: { type: "id", tableName: "myTable" },
  any: { type: "any" },
};

const unionOfObjectValidator: ValidatorJSON = {
  type: "union",
  value: [
    {
      type: "object",
      value: {
        kind: {
          fieldType: { type: "literal", value: "TEXT" },
          optional: false,
        },
        text: { fieldType: validators.string, optional: false },
      },
    },
    {
      type: "object",
      value: {
        kind: {
          fieldType: { type: "literal", value: "IMAGE" },
          optional: false,
        },
        uri: { fieldType: validators.string, optional: false },
      },
    },
  ],
};

const unionOfArrayValidator: ValidatorJSON = {
  type: "union",
  value: [
    {
      type: "array",
      value: validators.number,
    },
    {
      type: "array",
      value: validators.string,
    },
  ],
};

describe("walkAst", () => {
  describe("happy path", () => {
    const testHappyPath = (
      input: string,
      expected: Value,
      mode: "editField" | "addDocuments" | "editDocument" | "patchDocuments",
      allowTopLevelUndefined: boolean,
      validator?: ValidatorJSON,
    ) => {
      const { value: result, errors } = walkAst(input, {
        mode,
        allowTopLevelUndefined,
        validator,
      });
      expect(errors).toEqual([]);
      expect(result).toEqual(expected);

      // ensure that parsing the string version of a value produces the same result.
      const { value: roundTripResult, errors: roundTripErrors } = walkAst(
        stringifyValue(result),
        {
          mode,
        },
      );
      expect(roundTripResult).toEqual(result);
      expect(roundTripErrors).toEqual([]);
    };

    const testCases: {
      name: string;
      input: string;
      expected: Value;
      mode?: "editField" | "addDocuments" | "editDocument" | "patchDocuments";
      allowTopLevelUndefined?: boolean;
      validator?: ValidatorJSON;
    }[] = [
      {
        name: "wrapped in parens",
        input: "(({a: 2}))",
        expected: { a: 2 },
      },
      {
        name: "number",
        input: "1.25",
        expected: 1.25,
        validator: validators.number,
      },
      {
        name: "infinity",
        input: "Infinity",
        expected: Infinity,
        validator: validators.number,
      },
      {
        name: "infinity",
        input: "Infinity",
        expected: Infinity,
        validator: validators.number,
      },
      {
        name: "-infinity",
        input: "-Infinity",
        expected: -Infinity,
        validator: validators.number,
      },
      {
        name: "nan",
        input: "NaN",
        expected: NaN,
        validator: validators.number,
      },
      {
        name: "negative number",
        input: "-1.25",
        expected: -1.25,
        validator: validators.number,
      },
      {
        name: "bigint",
        input: "3n",
        expected: BigInt(3),
        validator: validators.bigint,
      },
      {
        name: "negative bigint",
        input: "-3n",
        expected: BigInt(-3),
        validator: validators.bigint,
      },
      {
        name: "boolean",
        input: "true",
        expected: true,
        validator: validators.boolean,
      },
      {
        name: "string",
        input: `"a string"`,
        expected: "a string",
        validator: validators.string,
      },
      {
        name: "single-quote string",
        input: "'a string'",
        expected: "a string",
        validator: validators.string,
      },
      {
        name: "null",
        input: "null",
        expected: null,
        validator: validators.null,
      },
      {
        name: "array",
        input: `[1,"abc",false]`,
        expected: [1, "abc", false],
        validator: {
          type: "array",
          value: {
            type: "union",
            value: [validators.number, validators.string, validators.boolean],
          },
        },
      },
      {
        name: "object",
        input: `{a: 1}`,
        expected: { a: 1 },
        validator: {
          type: "object",
          value: {
            a: { fieldType: validators.number, optional: false },
          },
        },
      },
      {
        name: "template literal",
        input: "`abc\ndef`",
        expected: "abc\ndef",
        validator: validators.string,
      },
      {
        name: "object with literal keys",
        input: `{"a": 1}`,
        expected: { a: 1 },
        validator: {
          type: "object",
          value: {
            a: { fieldType: validators.number, optional: false },
          },
        },
      },
      {
        name: "object with empty key",
        input: `{"": 1}`,
        expected: { "": 1 },
        validator: {
          type: "object",
          value: {
            "": { fieldType: validators.number, optional: false },
          },
        },
      },
      {
        name: "nested object",
        input: "{a: {b: 2}}",
        expected: { a: { b: 2 } },
        validator: {
          type: "object",
          value: {
            a: {
              fieldType: {
                type: "object",
                value: {
                  b: { fieldType: validators.number, optional: false },
                },
              },
              optional: false,
            },
          },
        },
      },
      {
        name: "nested object with system field",
        input: "{a: {_a: 1}}",
        expected: { a: { _a: 1 } },
        validator: {
          type: "object",
          value: {
            a: {
              fieldType: {
                type: "object",
                value: {
                  _a: { fieldType: validators.number, optional: false },
                },
              },
              optional: false,
            },
          },
        },
      },
      {
        name: "object with nested array",
        input: "{a: [1,2,3]}",
        expected: { a: [1, 2, 3] },
        validator: {
          type: "object",
          value: {
            a: {
              fieldType: {
                type: "array",
                value: validators.number,
              },
              optional: false,
            },
          },
        },
      },
      {
        name: "array with nested object",
        input: "[{a:1}]",
        expected: [{ a: 1 }],
        validator: {
          type: "array",
          value: {
            type: "object",
            value: {
              a: { fieldType: validators.number, optional: false },
            },
          },
        },
      },
      {
        name: "id",
        input: "'2wbatrp2sqy0ym0j6xra4agx9h059j0'",
        expected: "2wbatrp2sqy0ym0j6xra4agx9h059j0",
        validator: validators.id,
      },
      {
        name: "object with nested id",
        input: "{a: '2wbatrp2sqy0ym0j6xra4agx9h059j0'}",
        expected: { a: "2wbatrp2sqy0ym0j6xra4agx9h059j0" },
        validator: {
          type: "object",
          value: {
            a: { fieldType: validators.id, optional: false },
          },
        },
      },
      {
        name: "object with nested ids all over the place",
        input:
          "{a: '2wbatrp2sqy0ym0j6xra4agx9h059j0', b: ['31edfed94p29qg8byf54zm1s9h05xtg', '2z16se66x9g9jsgd759nb3fw9h00ymr']}",
        expected: {
          a: "2wbatrp2sqy0ym0j6xra4agx9h059j0",
          b: [
            "31edfed94p29qg8byf54zm1s9h05xtg",
            "2z16se66x9g9jsgd759nb3fw9h00ymr",
          ],
        },
        validator: {
          type: "object",
          value: {
            a: { fieldType: validators.id, optional: false },
            b: {
              fieldType: {
                type: "array",
                value: validators.id,
              },
              optional: false,
            },
          },
        },
      },
      {
        name: "top level undefined",
        allowTopLevelUndefined: true,
        input: "undefined",
        expected: UNDEFINED_PLACEHOLDER,
        // Even though the value is undefined, allowTopLevelUndefined is true, so the validator should bei gnored.
        validator: validators.number,
      },
      {
        name: "top level undefined in object",
        allowTopLevelUndefined: true,
        mode: "patchDocuments",
        input: "{a: undefined}",
        expected: { a: UNDEFINED_PLACEHOLDER },
        validator: {
          type: "object",
          value: {
            a: { fieldType: validators.number, optional: true },
          },
        },
      },
      {
        name: "top level union in object",
        mode: "patchDocuments",
        input: "{a: 123}",
        expected: { a: 123 },
        validator: {
          type: "union",
          value: [
            {
              type: "object",
              value: {
                a: { fieldType: validators.number, optional: false },
                b: { fieldType: validators.string, optional: false },
              },
            },
          ],
        },
      },
      {
        name: "undefined in union",
        input: "{}",
        expected: {},
        validator: {
          type: "union",
          value: [
            {
              type: "object",
              value: {
                a: { fieldType: validators.number, optional: true },
              },
            },
          ],
        },
      },
      {
        name: "multiline string with backticks",
        input: "`abc\\`\ndef`",
        expected: "abc`\ndef",
        validator: validators.string,
      },
      {
        name: "union of objects",
        input: `{kind: "TEXT", text: "abc"}`,
        expected: { kind: "TEXT", text: "abc" },
        validator: unionOfObjectValidator,
      },
      {
        name: "union of objects 2",
        input: `{kind: "IMAGE", uri: "abc"}`,
        expected: { kind: "IMAGE", uri: "abc" },
        validator: unionOfObjectValidator,
      },
      {
        name: "union of arrays",
        input: `[1,2,3]`,
        expected: [1, 2, 3],
        validator: unionOfArrayValidator,
      },
      {
        name: "union of arrays 2",
        input: `["a","b","c"]`,
        expected: ["a", "b", "c"],
        validator: unionOfArrayValidator,
      },
      {
        name: "object with any",
        input: `{a: 1, b: "abc"}`,
        expected: { a: 1, b: "abc" },
        validator: validators.any,
      },
      {
        name: "object with any key",
        input: `{a: 1, b: "abc"}`,
        expected: { a: 1, b: "abc" },
        validator: {
          type: "object",
          value: {
            a: { fieldType: validators.any, optional: false },
            b: { fieldType: validators.any, optional: false },
          },
        },
      },
      {
        name: "array with any",
        input: `[1, "abc"]`,
        expected: [1, "abc"],
        validator: validators.any,
      },
      {
        name: "array with any value",
        input: `[1, "abc"]`,
        expected: [1, "abc"],
        validator: { type: "array", value: validators.any },
      },
      {
        name: "any primitive",
        input: "1",
        expected: 1,
        validator: validators.any,
      },
      {
        name: "record validator",
        input: `{a: 1, b: "abc"}`,
        expected: { a: 1, b: "abc" },
        validator: {
          type: "record",
          keys: { type: "string" },
          values: {
            fieldType: {
              type: "union",
              value: [validators.number, validators.string],
            },
            optional: false,
          },
        },
      },
      {
        name: "record validator with union of ids",
        input: `{j9728bhc7wsqs6aptq2j6p4e496z8gbr: 1}`,
        expected: { j9728bhc7wsqs6aptq2j6p4e496z8gbr: 1 },
        validator: {
          type: "record",
          keys: {
            type: "union",
            value: [
              { type: "id", tableName: "abc" },
              { type: "id", tableName: "abc2" },
            ],
          },
          values: {
            fieldType: {
              type: "union",
              value: [validators.number, validators.string],
            },
            optional: false,
          },
        },
      },
    ];

    test.each(testCases)(
      "$name",
      ({
        input,
        expected,
        allowTopLevelUndefined = false,
        mode = "editField",
        validator = undefined,
      }) => {
        // Test with and without a validator.
        testHappyPath(input, expected, mode, allowTopLevelUndefined, validator);
        testHappyPath(input, expected, mode, allowTopLevelUndefined, undefined);
      },
    );
  });

  describe("error cases", () => {
    test.each([
      { name: "regex", input: "/a/" },
      { name: "invalid unary expression", input: "+3" },
      { name: "negative string", input: `-"a"` },
      { name: "negative object", input: `-{}` },
      { name: "unsupported constructor", input: "new User()" },
      { name: "Id class", input: "new Id()" },
      { name: "key starting with $", input: '{"$a": 1}' },
      { name: "non-identifier key", input: '{"foo-bar": 1}' },
      { name: "identifier", input: "a" },
      { name: "identifier in object", input: "{a: b}" },
      { name: "array with empty value", input: "[1,,3]" },
      // eslint-disable-next-line no-template-curly-in-string
      { name: "template literal with expression", input: "`${'abc'}`" },
      {
        name: "top level undefined",
        input: "undefined",
      },
      {
        name: "top level undefined in object",
        input: "{a: undefined}",
      },
      {
        name: "nested undefined in object even if top level is allowed",
        input: "{a: {b: undefined}}",
        allowTopLevelUndefined: true,
      },
      {
        name: "CallExpression that isn't bytes",
        input: "foo()",
      },
      {
        name: "Bytes with no arguments",
        input: "Bytes()",
      },
      {
        name: "Bytes with too many arguments",
        input: `Bytes("a", "a")`,
      },
      {
        name: "Bytes with bad argument",
        input: `Bytes("aaa")`,
      },
      { name: "-nan", input: "-NaN" },
      { name: "top level undefined in object", input: "{a: undefined}" },
    ])("$name", ({ input, allowTopLevelUndefined = false }) => {
      const { errors } = walkAst(input, {
        mode: "editField",
        allowTopLevelUndefined,
      });

      expect(errors).toMatchSnapshot();
    });
  });

  describe("top level system field disallowed", () => {
    const { errors } = walkAst('[{"_a": 1}]', { mode: "addDocuments" });
    expect(errors.length).toBeGreaterThan(0);
    expect(errors).toMatchSnapshot();
  });

  describe("validator error cases", () => {
    const testCases: {
      name: string;
      input: string;
      validator: ValidatorJSON;
      errorCode: SchemaValidationErrorCode;
      expectedValue?: Value;
    }[] = [
      {
        name: "Top-level literal mismatch",
        input: "'string'",
        expectedValue: "string",
        errorCode: "LiteralMismatch",
        validator: validators.number,
      },
      {
        name: "Top-level template literal",
        input: "`abc`",
        expectedValue: "abc",
        errorCode: "LiteralMismatch",
        validator: validators.number,
      },
      {
        name: "Object missing property",
        input: "{}",
        expectedValue: {},
        errorCode: "RequiredPropertyMissing",
        validator: {
          type: "object",
          value: {
            a: {
              fieldType: validators.number,
              optional: false,
            },
          },
        },
      },
      {
        name: "Extra property",
        input: "{a: 1, b: 2}",
        expectedValue: { a: 1, b: 2 },
        errorCode: "ExtraProperty",
        validator: {
          type: "object",
          value: { a: { fieldType: validators.number, optional: false } },
        },
      },
      {
        name: "Input is array, but validator is not",
        input: "[]",
        expectedValue: [],
        errorCode: "IsNotArray",
        validator: validators.string,
      },
      {
        name: "Input is object, but validator is not",
        input: "{}",
        expectedValue: {},
        errorCode: "IsNotObject",
        validator: validators.string,
      },
      {
        name: "Input is bytes, but validator is not",
        input: "Bytes('aaaa')",
        expectedValue: Base64.toByteArray("aaaa").buffer,
        errorCode: "IsNotBytes",
        validator: validators.string,
      },
      {
        name: "Literal in object",
        input: "{a: 1}",
        expectedValue: { a: 1 },
        errorCode: "LiteralMismatch",
        validator: {
          type: "object",
          value: {
            a: { fieldType: validators.string, optional: false },
          },
        },
      },
      {
        name: "Array with invalid element",
        input: "[1, 'abc']",
        expectedValue: [1, "abc"],
        errorCode: "LiteralMismatch",
        validator: {
          type: "array",
          value: validators.number,
        },
      },
      {
        name: "Object with invalid property value",
        input: "{ a: 'abc' }",
        expectedValue: { a: "abc" },
        errorCode: "LiteralMismatch",
        validator: {
          type: "object",
          value: {
            a: { fieldType: validators.number, optional: false },
          },
        },
      },
      {
        name: "Nested object with invalid property value",
        input: "{ a: { b: 'abc' } }",
        expectedValue: { a: { b: "abc" } },
        errorCode: "LiteralMismatch",
        validator: {
          type: "object",
          value: {
            a: {
              fieldType: {
                type: "object",
                value: {
                  b: { fieldType: validators.number, optional: false },
                },
              },
              optional: false,
            },
          },
        },
      },
      {
        name: "Array with invalid nested object",
        input: "[{ a: 'abc' }]",
        expectedValue: [{ a: "abc" }],
        errorCode: "LiteralMismatch",
        validator: {
          type: "array",
          value: {
            type: "object",
            value: {
              a: { fieldType: validators.number, optional: false },
            },
          },
        },
      },
      {
        name: "Object with invalid nested array",
        input: "{ a: ['abc', 123] }",
        expectedValue: { a: ["abc", 123] },
        errorCode: "LiteralMismatch",
        validator: {
          type: "object",
          value: {
            a: {
              fieldType: {
                type: "array",
                value: validators.number,
              },
              optional: false,
            },
          },
        },
      },
      {
        name: "Object with invalid nested id",
        input: "{ a: 'invalid-id' }",
        expectedValue: { a: "invalid-id" },
        errorCode: "LiteralMismatch",
        validator: {
          type: "object",
          value: {
            a: { fieldType: validators.id, optional: false },
          },
        },
      },
      {
        name: "union of objects",
        input: `{a: 1}`,
        expectedValue: { a: 1 },
        errorCode: "UnionMismatch",
        validator: unionOfObjectValidator,
      },
      {
        name: "union of arrays",
        input: `{a: 1}`,
        expectedValue: { a: 1 },
        errorCode: "UnionMismatch",
        validator: unionOfArrayValidator,
      },
      {
        name: "union of primitives",
        input: `"a"`,
        expectedValue: "a",
        errorCode: "LiteralMismatch",
        validator: {
          type: "union",
          value: [validators.number, validators.boolean],
        },
      },
      {
        name: "record keys are not ids",
        input: `{a: 1}`,
        expectedValue: { a: 1 },
        errorCode: "RecordKeysMismatch",
        validator: {
          type: "record",
          keys: { type: "id", tableName: "abc" },
          values: {
            fieldType: validators.number,
            optional: false,
          },
        },
      },
      {
        name: "record values do not match",
        input: `{a: 1}`,
        expectedValue: { a: 1 },
        errorCode: "LiteralMismatch",
        validator: {
          type: "record",
          keys: { type: "string" },
          values: {
            fieldType: validators.string,
            optional: false,
          },
        },
      },
      {
        name: "union with records inside",
        input: `{a: false}`,
        expectedValue: { a: false },
        errorCode: "UnionMismatch",
        validator: {
          type: "union",
          value: [
            {
              type: "record",
              keys: { type: "string" },
              values: {
                fieldType: validators.string,
                optional: false,
              },
            },
            {
              type: "record",
              keys: { type: "string" },
              values: {
                fieldType: validators.number,
                optional: false,
              },
            },
          ],
        },
      },
      {
        name: "record with invalid object inside",
        input: `{ key: {a: 1} }`,
        expectedValue: { key: { a: 1 } },
        errorCode: "LiteralMismatch",
        validator: {
          type: "record",
          keys: { type: "string" },
          values: {
            fieldType: {
              type: "object",
              value: {
                a: { fieldType: validators.string, optional: false },
              },
            },
            optional: false,
          },
        },
      },
    ];

    // TODO: Test that multiple errors can be produced.

    test.each(testCases)(
      "$name",
      ({ input, errorCode, validator, expectedValue }) => {
        const { value, errors } = walkAst(input, {
          validator,
          mode: "editField",
        });

        expect(errors).toEqual([
          expect.objectContaining({
            code: errorCode,
          }),
        ]);
        if (expectedValue !== undefined) {
          expect(value).toEqual(expectedValue);
        }
      },
    );
  });
});

describe("validateConvexFieldName", () => {
  test.each([
    {
      name: "starts with $",
      input: "$a",
      isTopLevel: false,
      expected: "Field cannot start with a '$'",
    },
    {
      name: "starts with _",
      input: "_a",
      isTopLevel: true,
      expected: "Field is top-level and cannot start with an underscore.",
    },
    {
      name: "nested starts with _",
      input: "_b",
      isTopLevel: false,
      expected: undefined,
    },
    {
      name: "contains non-ascii characters",
      input: "a\u0000",
      isTopLevel: false,
      expected: "Field must only contain non-control ASCII characters.",
    },
  ])(
    "returns the correct error message for a $name",
    ({ input, isTopLevel, expected }) => {
      const returned = validateConvexFieldName(input, "Field", isTopLevel);
      expect(returned).toEqual(expected);
    },
  );
});
