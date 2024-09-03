import { GenericId } from "../values/index.js";
import { describe, test } from "vitest";
import { assert, Equals } from "../test/type_testing.js";
import { v, Infer } from "../values/validator.js";

describe("Validators", () => {
  test("optional types don't lose specificity", () => {
    const a = v.optional(v.id("a"));
    const b = v.optional(v.null());
    const c = v.optional(v.number());
    const d = v.optional(v.int64());
    const e = v.optional(v.boolean());
    const f = v.optional(v.string());
    const g = v.optional(v.bytes());
    const h = v.optional(v.literal("a"));
    const i = v.optional(v.array(v.string()));
    const j = v.optional(v.object({ a: v.string() }));
    const k = v.optional(v.record(v.string(), v.string()));
    const l = v.optional(v.union(v.string(), v.number()));

    // Optional makes types a union with undefined.
    assert<Equals<Infer<typeof a>, GenericId<"a"> | undefined>>();
    assert<Equals<Infer<typeof b>, null | undefined>>();
    assert<Equals<Infer<typeof c>, number | undefined>>();
    assert<Equals<Infer<typeof d>, bigint | undefined>>();
    assert<Equals<Infer<typeof e>, boolean | undefined>>();
    assert<Equals<Infer<typeof f>, string | undefined>>();
    assert<Equals<Infer<typeof g>, ArrayBuffer | undefined>>();
    assert<Equals<Infer<typeof h>, "a" | undefined>>();
    assert<Equals<Infer<typeof i>, string[] | undefined>>();
    assert<Equals<Infer<typeof j>, { a: string } | undefined>>();
    assert<Equals<Infer<typeof k>, Record<string, string> | undefined>>();
    assert<Equals<Infer<typeof l>, string | number | undefined>>();

    // Note: this test does not actually verify this property unless
    // the tsconfig.json option `"exactOptionalPropertyTypes": true` is used.
    const _optionals = v.object({ a, b, c, d, e, f, g, h, i, j, k, l });
    assert<
      Equals<
        Infer<typeof _optionals>,
        {
          a?: GenericId<"a">;
          b?: null;
          c?: number;
          d?: bigint;
          e?: boolean;
          f?: string;
          g?: ArrayBuffer;
          h?: "a";
          i?: string[];
          j?: { a: string };
          k?: Record<string, string>;
          l?: string | number;
        }
      >
    >();
  });

  test("Most validators don't accept optional validators as children", () => {
    const optional = v.optional(v.string());
    const required = v.string();

    v.object({ optional });

    v.array(required);
    // @ts-expect-error This should be an error
    v.array(optional);

    v.record(required, required);
    // @ts-expect-error This should be an error
    v.record(required, optional);
    // @ts-expect-error This should be an error
    v.record(optional, required);
    // @ts-expect-error This should be an error
    v.record(optional, optional);

    v.union(required, required);
    // @ts-expect-error This should be an error
    v.union(optional, optional);
    // @ts-expect-error This should be an error
    v.union(required, optional);
    // @ts-expect-error This should be an error
    v.union(optional, required);
  });

  test("complex types look good", () => {
    const _obj = v.object({
      a: v.record(v.string(), v.string()),
      b: v.string(),
      c: v.union(v.string(), v.union(v.string(), v.number())),
      d: v.object({ foo: v.string(), bar: v.optional(v.number()) }),
    });

    type Expected = {
      a: Record<string, string>;
      b: string;
      c: string | number;
      d: {
        bar?: number | undefined;
        foo: string;
      };
    };
    assert<Equals<Infer<typeof _obj>, Expected>>();
  });
});
