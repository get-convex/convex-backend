/**
 * @vitest-environment custom-vitest-environment.ts
 */

/* eslint-disable @typescript-eslint/no-unused-vars */
import { test, describe, expectTypeOf } from "vitest";
import { anyApi } from "../server/api.js";

import type { ApiFromModules, QueryBuilder } from "../server/index.js";
import { useQuery as useQueryReal, type UseQueryResult } from "./client.js";

const useQuery = (() => {}) as unknown as typeof useQueryReal;
const query: QueryBuilder<any, "public"> = (() => {}) as any;

const module = {
  noArgs: query(() => "result"),
  args: query((_ctx, { _arg }: { _arg: string }) => "result"),
};
type API = ApiFromModules<{ module: typeof module }>;
const api = anyApi as unknown as API;

describe("useQuery object-form result types", () => {
  test("returns UseQueryResult for object form", () => {
    const result = useQuery({
      query: api.module.args,
      args: { _arg: "asdf" },
    });
    expectTypeOf(result).toEqualTypeOf<UseQueryResult<string>>();

    const resultThrow = useQuery({
      query: api.module.args,
      args: { _arg: "asdf" },
      throwOnError: true,
    });
    expectTypeOf(resultThrow).toEqualTypeOf<UseQueryResult<string>>();

    const _arg: string | undefined = undefined;
    const conditionalResult = useQuery(
      !_arg
        ? "skip"
        : {
            query: api.module.args,
            args: { _arg },
          },
    );
    expectTypeOf(conditionalResult).toEqualTypeOf<UseQueryResult<string>>();
  });
});
