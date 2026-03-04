import { renderHook } from "@testing-library/react";
import { Shape } from "shapes";
import { GenericDocument } from "convex/server";
import { useTableFields } from "@common/features/data/components/Table/utils/useTableFields";
import { SchemaJson } from "@common/lib/format";

describe("useTableFields", () => {
  it("returns top-level fields for a regular object shape", () => {
    const shape: Shape = {
      type: "Object",
      fields: [
        { fieldName: "_id", optional: false, shape: { type: "String" } },
        { fieldName: "name", optional: false, shape: { type: "String" } },
        {
          fieldName: "_creationTime",
          optional: false,
          shape: { type: "Float64", float64Range: {} },
        },
      ],
    };

    const { result } = renderHook(() =>
      useTableFields("test", shape, null, []),
    );

    expect(result.current).toEqual(["_id", "name", "_creationTime"]);
  });

  it("includes observed document fields when shape is null", () => {
    const data: GenericDocument[] = [
      {
        _id: "jd7n53z7h5d4x9r7m0s9m5d7ch7f8yq0" as GenericDocument["_id"],
        _creationTime: 1700000000000,
        "a?b": true,
        "a=b": true,
      },
    ];

    const { result } = renderHook(() =>
      useTableFields("test", null, null, data),
    );

    expect(result.current).toEqual(["_id", "a=b", "a?b", "_creationTime"]);
  });

  it("falls back to observed document fields when shape has no top-level keys", () => {
    const shape: Shape = {
      type: "Record",
      keyShape: { type: "String" },
      valueShape: {
        optional: false,
        shape: { type: "Boolean" },
      },
    };
    const data: GenericDocument[] = [
      {
        _id: "jd7n53z7h5d4x9r7m0s9m5d7ch7f8yq0" as GenericDocument["_id"],
        _creationTime: 1700000000000,
        "a?b": true,
        "a=b": true,
      },
    ];

    const { result } = renderHook(() =>
      useTableFields("test", shape, null, data),
    );

    expect(result.current).toEqual(["_id", "a=b", "a?b", "_creationTime"]);
  });

  it("skips observed document fields when shape already has fields", () => {
    const shape: Shape = {
      type: "Object",
      fields: [
        { fieldName: "_id", optional: false, shape: { type: "String" } },
        { fieldName: "name", optional: false, shape: { type: "String" } },
        {
          fieldName: "_creationTime",
          optional: false,
          shape: { type: "Float64", float64Range: {} },
        },
      ],
    };

    const initialData: GenericDocument[] = [
      {
        _id: "jd7n53z7h5d4x9r7m0s9m5d7ch7f8yq0" as GenericDocument["_id"],
        _creationTime: 1700000000000,
        name: "Ada",
      },
    ];
    const nextData: GenericDocument[] = [
      {
        _id: "jd7n53z7h5d4x9r7m0s9m5d7ch7f8yq0" as GenericDocument["_id"],
        _creationTime: 1700000000000,
        name: "Ada",
        extraFromData: true,
      },
    ];

    const { result, rerender } = renderHook(
      ({ data }: { data: GenericDocument[] }) =>
        useTableFields("test", shape, null, data),
      {
        initialProps: { data: initialData },
      },
    );

    expect(result.current).toEqual(["_id", "name", "_creationTime"]);
    const initialFields = result.current;

    rerender({ data: nextData });

    // using toBe to do a strict equality check (in this case,
    // the change in `data` doesn’t matter, so we want to avoid
    // recomputing the fields)
    expect(result.current).toBe(initialFields);
    expect(result.current).toEqual(["_id", "name", "_creationTime"]);
  });

  it("uses complete schema fields when schema validation is enforced", () => {
    const shape: Shape = {
      type: "Record",
      keyShape: { type: "String" },
      valueShape: {
        optional: false,
        shape: { type: "Boolean" },
      },
    };
    const data: GenericDocument[] = [
      {
        _id: "jd7n53z7h5d4x9r7m0s9m5d7ch7f8yq0" as GenericDocument["_id"],
        _creationTime: 1700000000000,
        name: "Ada",
        extraFromData: true,
      },
    ];
    const activeSchema: SchemaJson = {
      schemaValidation: true,
      tables: [
        {
          tableName: "test",
          documentType: {
            type: "object",
            value: {
              name: {
                fieldType: { type: "string" },
                optional: false,
              },
            },
          },
          indexes: [],
          searchIndexes: [],
        },
      ],
    };

    const { result } = renderHook(() =>
      useTableFields("test", shape, activeSchema, data),
    );

    expect(result.current).toEqual(["_id", "name", "_creationTime"]);
  });

  it("preserves partial schema-derived fields while shape is loading", () => {
    const activeSchema: SchemaJson = {
      schemaValidation: false,
      tables: [
        {
          tableName: "test",
          documentType: {
            type: "union",
            value: [
              {
                type: "object",
                value: {
                  name: {
                    fieldType: { type: "string" },
                    optional: false,
                  },
                },
              },
              { type: "string" },
            ],
          },
          indexes: [],
          searchIndexes: [],
        },
      ],
    };

    const { result } = renderHook(() =>
      useTableFields("test", null, activeSchema, []),
    );

    expect(result.current).toEqual(["_id", "name", "_creationTime"]);
  });
});
