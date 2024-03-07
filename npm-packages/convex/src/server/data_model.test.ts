import { test } from "@jest/globals";
import { GenericId } from "../values/index.js";
import { FieldTypeFromFieldPath } from "./data_model.js";
import { assert, Equals } from "../test/type_testing.js";

/**
 * Compile time tests to make sure our data model types are doing what we
 * expect.
 */
type Document = {
  _id: GenericId<"tableName">;
  stringField: string;
  nestedObject: {
    numberField: number;
    doublyNestedObject: {
      booleanField: boolean;
    };
  };
  optionalField?: string;
};

// This method is doing some fancy inference so test it carefully.
test("FieldTypeFromFieldPath allows accessing all the primitives in the document", () => {
  assert<
    Equals<FieldTypeFromFieldPath<Document, "_id">, GenericId<"tableName">>
  >();
  assert<Equals<FieldTypeFromFieldPath<Document, "stringField">, string>>();
  assert<
    Equals<FieldTypeFromFieldPath<Document, "nestedObject.numberField">, number>
  >();
  assert<
    Equals<
      FieldTypeFromFieldPath<
        Document,
        "nestedObject.doublyNestedObject.booleanField"
      >,
      boolean
    >
  >();
});

test("FieldTypeFromFieldPath includes `undefined` if the fields is optional", () => {
  assert<
    Equals<
      FieldTypeFromFieldPath<Document, "optionalField">,
      string | undefined
    >
  >();
});

test("FieldTypeFromFieldPath resolves to undefined if it can't find a field", () => {
  assert<Equals<FieldTypeFromFieldPath<Document, "missingField">, undefined>>();
  assert<
    Equals<
      FieldTypeFromFieldPath<Document, "missingField.nestedField">,
      undefined
    >
  >();
  assert<
    Equals<
      FieldTypeFromFieldPath<Document, "stringField.nestedField">,
      undefined
    >
  >();
  assert<Equals<FieldTypeFromFieldPath<Document, "">, undefined>>();
});

type DocumentWithUnion =
  | {
      _id: GenericId<"tableName">;
      _creationTime: number;
      tag: "A";
      stringField: string;
      a: string;
      nestedObject: {
        numberField: number;
      };
    }
  | {
      _id: GenericId<"tableName">;
      _creationTime: number;
      tag: "B";
      stringField: string;
      b: number;
    };

test("FieldTypeFromFieldPath with unions", () => {
  assert<
    Equals<FieldTypeFromFieldPath<DocumentWithUnion, "a">, string | undefined>
  >();
  assert<
    Equals<FieldTypeFromFieldPath<DocumentWithUnion, "b">, number | undefined>
  >();
  assert<Equals<FieldTypeFromFieldPath<DocumentWithUnion, "tag">, "A" | "B">>();
  assert<
    Equals<
      FieldTypeFromFieldPath<DocumentWithUnion, "nestedObject.numberField">,
      number | undefined
    >
  >();
});
