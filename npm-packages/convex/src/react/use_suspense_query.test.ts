/**
 * @vitest-environment custom-vitest-environment.ts
 */

/* eslint-disable @typescript-eslint/no-unused-vars */
import { test, describe, expectTypeOf } from "vitest";
import { anyApi } from "../server/api.js";

import type { ApiFromModules, QueryBuilder } from "../server/index.js";
import type { ConvexReactClient } from "./client.js";
import { useSuspenseQuery as useSuspenseQueryReal } from "./client.js";

const useSuspenseQuery = (() => {}) as unknown as typeof useSuspenseQueryReal;

const query: QueryBuilder<any, "public"> = (() => {}) as any;

const module = {
  noArgs: query(() => "result"),
  args: query((_ctx, { _arg }: { _arg: string }) => "result"),
};
type API = ApiFromModules<{ module: typeof module }>;
const api = anyApi as unknown as API;
const client = {} as ConvexReactClient;

describe("useSuspenseQuery types", () => {
  test("supports positional and object options", () => {
    useSuspenseQuery(api.module.noArgs, {});
    useSuspenseQuery(api.module.noArgs);

    useSuspenseQuery({
      query: api.module.args,
      args: { _arg: "asdf" },
      client,
      requireAuth: true,
    });

    // @ts-expect-error
    useSuspenseQuery({
      query: api.module.args,
      args: { _arg: "asdf" },
      throwOnError: true,
    });

    const suspenseResult = useSuspenseQuery(api.module.args, { _arg: "asdf" });
    expectTypeOf(suspenseResult).toEqualTypeOf<string | undefined>();

    const _arg: string | undefined = undefined;
    const maybeResult = useSuspenseQuery(
      !_arg
        ? "skip"
        : {
            query: api.module.args,
            args: { _arg },
          },
    );
    expectTypeOf(maybeResult).toEqualTypeOf<string | undefined>();

    const maybeResultPositional = useSuspenseQuery(
      api.module.args,
      !_arg ? "skip" : { _arg },
    );
    expectTypeOf(maybeResultPositional).toEqualTypeOf<string | undefined>();

    useSuspenseQuery("skip");
  });
});
