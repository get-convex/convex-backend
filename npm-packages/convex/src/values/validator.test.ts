/* eslint-disable @typescript-eslint/ban-types */
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

    // Optional has no effect when not in an object.
    assert<Equals<Infer<typeof a>, GenericId<"a">>>();
    assert<Equals<Infer<typeof b>, null>>();
    assert<Equals<Infer<typeof c>, number>>();
    assert<Equals<Infer<typeof d>, bigint>>();
    assert<Equals<Infer<typeof e>, boolean>>();
    assert<Equals<Infer<typeof f>, string>>();
    assert<Equals<Infer<typeof g>, ArrayBuffer>>();
    assert<Equals<Infer<typeof h>, "a">>();
    assert<Equals<Infer<typeof i>, string[]>>();
    assert<Equals<Infer<typeof j>, { a: string }>>();
    assert<Equals<Infer<typeof k>, Record<string, string>>>();
    assert<Equals<Infer<typeof l>, string | number>>();

    // Optional
    const optionals = v.object({ a, b, c, d, e, f, g, h, i, j, k, l });
    assert<
      Equals<
        Infer<typeof optionals>,
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

  test("complex types look good", () => {
    const obj = v.object({
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
    assert<Equals<Infer<typeof obj>, Expected>>();
  });
});
