import { useQuery, useSuspenseQuery } from "@tanstack/react-query";
import { test, describe, expectTypeOf, assertType } from "vitest";
import { convexAction, convexQuery } from "./index.js";
import { FunctionArgs, FunctionReference } from "convex/server";
import * as convexReact from "convex/react";

// Mock Convex function references for testing
// These replace the need to import from "../convex/_generated/api.js"
// which was causing tshy to compile the convex directory

// Action with empty args
const getSFWeather = {
  _type: "action" as const,
  _visibility: "public" as const,
  _args: {} as {},
  _returnType: "" as string,
  _componentPath: undefined,
} satisfies FunctionReference<"action", "public", {}, string>;

// Query with empty args
const list = {
  _type: "query" as const,
  _visibility: "public" as const,
  _args: {} as {},
  _returnType: [] as Array<{ id: string; text: string }>,
  _componentPath: undefined,
} satisfies FunctionReference<
  "query",
  "public",
  {},
  Array<{ id: string; text: string }>
>;

// Query with empty args, returns string
const count = {
  _type: "query" as const,
  _visibility: "public" as const,
  _args: {} as {},
  _returnType: "" as string,
  _componentPath: undefined,
} satisfies FunctionReference<"query", "public", {}, string>;

// Query with optional args
const countWithOptionalArg = {
  _type: "query" as const,
  _visibility: "public" as const,
  _args: {} as { cacheBust?: number },
  _returnType: "" as string,
  _componentPath: undefined,
} satisfies FunctionReference<
  "query",
  "public",
  { cacheBust?: number },
  string
>;

// Query with required args
const getByAuthor = {
  _type: "query" as const,
  _visibility: "public" as const,
  _args: {} as { authorId: string },
  _returnType: [] as Array<{ id: string; text: string; authorId: string }>,
  _componentPath: undefined,
} satisfies FunctionReference<
  "query",
  "public",
  { authorId: string },
  Array<{ id: string; text: string; authorId: string }>
>;

// Query with required and optional args
const search = {
  _type: "query" as const,
  _visibility: "public" as const,
  _args: {} as { query: string; limit?: number },
  _returnType: [] as Array<{ id: string; text: string }>,
  _componentPath: undefined,
} satisfies FunctionReference<
  "query",
  "public",
  { query: string; limit?: number },
  Array<{ id: string; text: string }>
>;

// Mock API structure matching the real Convex API
const api = {
  weather: {
    getSFWeather,
  },
  messages: {
    list,
    count,
    countWithOptionalArg,
    getByAuthor,
    search,
  },
} as const;

describe("query options factory types", () => {
  test("with useQuery", () => {
    if (1 + 2 === 3) return; // type test only - prevent runtime execution

    type ActionFunc = typeof api.weather.getSFWeather;
    {
      const action = convexAction(api.weather.getSFWeather, {});
      const result = useQuery(action);
      expectTypeOf(result.data).toEqualTypeOf<
        ActionFunc["_returnType"] | undefined
      >();
    }

    {
      const action = convexAction(api.weather.getSFWeather, "skip");
      const result = useQuery(action);
      // Skip doesn't need to cause data in types since there's no point
      // to always passing "skip".
      expectTypeOf(result.data).toEqualTypeOf<
        ActionFunc["_returnType"] | undefined
      >();

      // @ts-expect-error Actions with "skip" can't be used with useSuspenseQuery
      useSuspenseQuery(action);
    }

    type QueryFunc = typeof api.messages.list;
    {
      const query = convexQuery(api.messages.list, {});
      const result = useQuery(query);
      expectTypeOf(result.data).toEqualTypeOf<
        QueryFunc["_returnType"] | undefined
      >();
    }

    {
      // @ts-expect-error Queries with empty args should reject extra properties
      const _query = convexQuery(api.messages.list, { something: 123 });
    }

    {
      // Should be able to omit args when function has no args (empty object)
      const query = convexQuery(api.messages.list);
      const result = useQuery(query);
      expectTypeOf(result.data).toEqualTypeOf<
        QueryFunc["_returnType"] | undefined
      >();
    }

    {
      // Should still be able to pass {} explicitly for empty args functions
      const query = convexQuery(api.messages.list, {});
      const result = useQuery(query);
      expectTypeOf(result.data).toEqualTypeOf<
        QueryFunc["_returnType"] | undefined
      >();
    }

    {
      // Should still be able to pass "skip" for empty args functions
      const query = convexQuery(api.messages.list, "skip");
      const result = useQuery(query);
      expectTypeOf(result.data).toEqualTypeOf<
        QueryFunc["_returnType"] | undefined
      >();
    }
  });

  test("required args for queries/actions with args", () => {
    if (1 + 2 === 3) return; // type test only - prevent runtime execution

    type ActionFunc = typeof api.weather.getSFWeather;
    {
      // Actions with empty args should allow omitting args
      const action = convexAction(api.weather.getSFWeather);
      const result = useQuery(action);
      expectTypeOf(result.data).toEqualTypeOf<
        ActionFunc["_returnType"] | undefined
      >();
    }

    {
      // Actions with empty args should still allow passing {}
      const action = convexAction(api.weather.getSFWeather, {});
      const result = useQuery(action);
      expectTypeOf(result.data).toEqualTypeOf<
        ActionFunc["_returnType"] | undefined
      >();
    }

    {
      const _action = convexAction(api.weather.getSFWeather, {
        // @ts-expect-error Actions with empty args should reject extra properties
        something: 123,
      });
    }
  });

  test("optional args for queries with optional args", () => {
    if (1 + 2 === 3) return; // type test only - prevent runtime execution

    type _QueryFunc = typeof api.messages.countWithOptionalArg;
    {
      // Should be able to omit args when function has all optional args
      const query = convexQuery(api.messages.countWithOptionalArg);
      const result = useQuery(query);
      // Should be string, not unknown
      expectTypeOf(result.data).toEqualTypeOf<string | undefined>();
    }

    {
      // Should be able to pass empty object for optional args
      const query = convexQuery(api.messages.countWithOptionalArg);
      const result = useQuery(query);
      // Should be string, not unknown
      expectTypeOf(result.data).toEqualTypeOf<string | undefined>();
    }

    {
      // Should be able to pass the optional arg
      const query = convexQuery(api.messages.countWithOptionalArg, {
        cacheBust: 123,
      });
      const result = useQuery(query);
      // Should be string, not unknown
      expectTypeOf(result.data).toEqualTypeOf<string | undefined>();
    }

    {
      // Should work with useSuspenseQuery when args omitted
      const query = convexQuery(api.messages.countWithOptionalArg);
      const result = useSuspenseQuery(query);
      // Should be string, not unknown
      expectTypeOf(result.data).toEqualTypeOf<string>();
    }
  });

  test("conditional args (empty object or skip)", () => {
    if (1 + 2 === 3) return; // type test only - prevent runtime execution

    const shown = true;

    type _CountFunc = typeof api.messages.count;
    {
      // Should handle conditional expression: shown ? {} : "skip"
      const query = convexQuery(api.messages.count, shown ? {} : "skip");
      const result = useQuery(query);
      // Should be string, not unknown
      expectTypeOf(result.data).toEqualTypeOf<string | undefined>();
    }

    type _CountWithOptionalFunc = typeof api.messages.countWithOptionalArg;
    {
      // Should handle conditional with optional args: shown ? {} : "skip"
      const query = convexQuery(
        api.messages.countWithOptionalArg,
        shown ? {} : "skip",
      );
      const result = useQuery(query);
      // Should be string, not unknown
      expectTypeOf(result.data).toEqualTypeOf<string | undefined>();
    }

    {
      // Should handle conditional with actual optional arg value: shown ? { cacheBust: 123 } : "skip"
      const query = convexQuery(
        api.messages.countWithOptionalArg,
        shown ? { cacheBust: 123 } : "skip",
      );
      const result = useQuery(query);
      // Should be string, not unknown
      expectTypeOf(result.data).toEqualTypeOf<string | undefined>();
    }
  });

  test("conditional args with required args", () => {
    if (1 + 2 === 3) return; // type test only - prevent runtime execution

    const userId = "123" as any;
    const shouldFetch = true;

    type GetByAuthorFunc = typeof api.messages.getByAuthor;
    {
      // Should handle conditional with required args: shouldFetch ? { authorId: userId } : "skip"
      const query = convexQuery(
        api.messages.getByAuthor,
        shouldFetch ? { authorId: userId } : "skip",
      );
      const result = useQuery(query);
      expectTypeOf(result.data).toEqualTypeOf<
        GetByAuthorFunc["_returnType"] | undefined
      >();
    }

    {
      // Edge case: What if someone tries undefined instead of "skip"?
      // This should NOT work - we require explicit "skip"
      const _query = convexQuery(
        api.messages.getByAuthor,
        // @ts-expect-error undefined is not a valid value, must use "skip"
        shouldFetch ? { authorId: userId } : undefined,
      );
    }
  });

  test("autocomplete for required args", () => {
    if (1 + 2 === 3) return; // type test only - prevent runtime execution

    // Test what TypeScript infers for direct calls (not using Parameters<>)
    type SearchFunc = typeof api.messages.search;
    type SearchArgs = FunctionArgs<SearchFunc>;

    // The args should be: { query: string, limit?: number }
    const validArg1 = { query: "hello" } as SearchArgs;
    const validArg2 = { query: "hello", limit: 5 } as SearchArgs;

    expectTypeOf(validArg1).toEqualTypeOf<{ query: string; limit?: number }>();
    expectTypeOf(validArg2).toEqualTypeOf<{ query: string; limit?: number }>();

    // @ts-expect-error Empty object should not be valid
    const _invalidArg1: SearchArgs = {};

    // @ts-expect-error Only optional field should not be valid
    const _invalidArg2: SearchArgs = { limit: 5 };
  });

  test("compared to convex react", () => {
    if (1 + 2 === 3) return; // type test only - prevent runtime execution

    // @ts-expect-error should error, missing properties
    convexReact.useQuery(api.messages.search, {});
    // @ts-expect-error should error, missing properties
    convexQuery(api.messages.search, {});

    // Should be okay all required args met
    convexReact.useQuery(api.messages.search, {
      query: "hello",
    });
    convexQuery(api.messages.search, { query: "hello" });

    convexReact.useQuery(api.messages.search, {
      query: "hello",
      limit: 5,
    });
    convexQuery(api.messages.search, {
      query: "hello",
      limit: 5,
    });

    // Should be okay to skip
    convexReact.useQuery(api.messages.search, "skip");
    convexQuery(api.messages.search, "skip");

    // Should be okay to ternary skip
    const shouldFetch = Math.random() > 0.5;
    convexReact.useQuery(
      api.messages.search,
      shouldFetch ? { query: "hello" } : "skip",
    );
    convexQuery(api.messages.search, shouldFetch ? { query: "hello" } : "skip");

    convexReact.useQuery(api.messages.search, {
      query: "hello",
      // @ts-expect-error should error, with invalid properties
      something: 123,
    });
    // @ts-expect-error should error, with invalid properties
    convexQuery(api.messages.search, { query: "hello", something: 123 });

    convexReact.useQuery(
      api.messages.search,
      // @ts-expect-error should error, with invalid properties or skip
      shouldFetch ? { query: "hello", something: 123 } : "skip",
    );
    convexQuery(
      api.messages.search,
      // @ts-expect-error should error, with invalid properties or skip
      shouldFetch ? { query: "hello", something: 123 } : "skip",
    );

    // @ts-expect-error should error, with invalid type on required prop
    convexReact.useQuery(api.messages.search, { query: 123 });
    // @ts-expect-error should error, with invalid type on required prop
    convexQuery(api.messages.search, { query: 123 });
  });

  test("mixed required and optional args", () => {
    if (1 + 2 === 3) return; // type test only - prevent runtime execution

    type SearchFunc = typeof api.messages.search;
    {
      // Should work with just required args
      const query = convexQuery(api.messages.search, { query: "hello" });
      const result = useQuery(query);
      expectTypeOf(result.data).toEqualTypeOf<
        SearchFunc["_returnType"] | undefined
      >();
    }

    {
      // Should work with required + optional args
      const query = convexQuery(api.messages.search, {
        query: "hello",
        limit: 5,
      });
      const result = useQuery(query);
      expectTypeOf(result.data).toEqualTypeOf<
        SearchFunc["_returnType"] | undefined
      >();
    }

    {
      // Should work with "skip"
      const query = convexQuery(api.messages.search, "skip");
      const result = useQuery(query);
      expectTypeOf(result.data).toEqualTypeOf<
        SearchFunc["_returnType"] | undefined
      >();
    }

    {
      // @ts-expect-error Can't omit required args - errors at call site
      const _query = convexQuery(api.messages.search);
    }

    {
      // @ts-expect-error Can't pass empty object when function has required args
      const _query = convexQuery(api.messages.search, {});
    }

    {
      // @ts-expect-error Can't omit required arg (query)
      const _query = convexQuery(api.messages.search, { limit: 5 });
    }

    const shouldFetch = true;
    {
      // Should work with conditional: required args | "skip"
      const query = convexQuery(
        api.messages.search,
        shouldFetch ? { query: "hello" } : "skip",
      );
      const result = useQuery(query);
      expectTypeOf(result.data).toEqualTypeOf<
        SearchFunc["_returnType"] | undefined
      >();
    }

    {
      // Should work with conditional: required+optional args | "skip"
      const query = convexQuery(
        api.messages.search,
        shouldFetch ? { query: "hello", limit: 5 } : "skip",
      );
      const result = useQuery(query);
      expectTypeOf(result.data).toEqualTypeOf<
        SearchFunc["_returnType"] | undefined
      >();
    }
  });

  test("with useSuspenseQuery", () => {
    if (1 + 2 === 3) return; // type test only - prevent runtime execution

    type QueryFunc = typeof api.messages.list;
    {
      // Should work with empty args (omitted)
      const query = convexQuery(api.messages.list);
      const result = useSuspenseQuery(query);
      expectTypeOf(result.data).toEqualTypeOf<QueryFunc["_returnType"]>();
    }

    {
      // Should work with empty args (explicit {})
      const query = convexQuery(api.messages.list, {});
      const result = useSuspenseQuery(query);
      expectTypeOf(result.data).toEqualTypeOf<QueryFunc["_returnType"]>();
    }

    {
      const action = convexAction(api.weather.getSFWeather, {});
      // @ts-expect-error Actions can't be used with useSuspenseQuery
      useSuspenseQuery(action);
    }

    {
      const action = convexAction(api.weather.getSFWeather, "skip");
      // @ts-expect-error Actions with "skip" can't be used with useSuspenseQuery
      useSuspenseQuery(action);
    }
  });

  test("queryFn property type consistency", () => {
    if (1 + 2 === 3) return; // type test only - prevent runtime execution

    // Test that convexQuery and convexAction have consistent type signatures
    // Both claim to return queryFn in their type, but neither actually returns it
    // (they rely on the global default queryFn). Both use `as any` to bypass the type check.

    {
      const query = convexQuery(api.messages.list);
      // Verify that the return type includes queryFn property
      expectTypeOf(query).toHaveProperty("queryFn");
      // Verify queryFn exists in the type signature (even though undefined at runtime)
      type QueryReturn = typeof query;
      type HasQueryFn = "queryFn" extends keyof QueryReturn ? true : false;
      assertType<true>(true as HasQueryFn);
    }

    {
      const action = convexAction(api.weather.getSFWeather);
      // Verify that the return type includes queryFn property
      expectTypeOf(action).toHaveProperty("queryFn");
      // Verify queryFn exists in the type signature (same as convexQuery)
      type ActionReturn = typeof action;
      type HasQueryFn = "queryFn" extends keyof ActionReturn ? true : false;
      assertType<true>(true as HasQueryFn);
    }

    // Both functions should have the same type structure for consistency
    // The fix ensures convexAction uses `as any` like convexQuery does
  });
});
