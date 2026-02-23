/**
 * @vitest-environment custom-vitest-environment.ts
 */

/* eslint-disable @typescript-eslint/no-unused-vars */
import { test, describe, expectTypeOf } from "vitest";
import { anyApi, makeFunctionReference } from "../server/api.js";

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

describe("useQuery object options types", () => {
  test("supports object options and skip sentinel", () => {
    useQuery({
      query: api.module.noArgs,
      args: {},
    });

    useQuery({
      query: api.module.args,
      args: { _arg: "asdf" },
    });

    const _arg: string | undefined = undefined;
    useQuery(
      !_arg
        ? "skip"
        : {
            query: api.module.args,
            args: { _arg },
          },
    );

    const result = useQuery({
      query: api.module.args,
      args: { _arg: "asdf" },
    });
    expectTypeOf(result).toEqualTypeOf<UseQueryResult<string>>();

    useQuery("skip");
  });

  test("rejects wrong arg types", () => {
    // @ts-expect-error wrong arg shape
    useQuery({
      query: api.module.args,
      args: { wrongField: "asdf" },
    });

    // @ts-expect-error missing required args field
    useQuery({
      query: api.module.args,
    });

    // @ts-expect-error mutation reference is not a query
    useQuery({
      query: makeFunctionReference<"mutation", Record<string, never>, void>(
        "myMutation",
      ),
      args: {},
    });
  });
});
