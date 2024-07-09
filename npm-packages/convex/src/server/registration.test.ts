import { test, describe, expect } from "vitest";
import { assert, Equals } from "../test/type_testing.js";
import { v } from "../values/validator.js";
import {
  ActionBuilder,
  ApiFromModules,
  DefaultFunctionArgs,
  QueryBuilder,
  actionGeneric,
  mutationGeneric,
  queryGeneric,
} from "./index.js";
import { EmptyObject, MutationBuilder } from "./registration.js";

describe("argument inference", () => {
  // Test with mutation, but all the wrappers work the same way.
  const mutation: MutationBuilder<any, "public"> = (() => {
    // Intentional noop. We're only testing the type
  }) as any;

  const module = {
    inlineNoArg: mutation(() => "result"),
    inlineUntypedArg: mutation((_ctx, { _arg }) => "result"),
    inlineTypedArg: mutation((_ctx, { _arg }: { _arg: string }) => "result"),

    // There are unusual syntaxes.
    inlineUntypedDefaultArg: mutation(
      (_ctx, { _arg } = { _arg: 1 }) => "result",
    ),
    inlineTypedDefaultArg: mutation(
      // @ts-expect-error This syntax has never been allowed.
      (_ctx, { _arg }: { _arg: string } = { _arg: "default" }) => "result",
    ),
    inlineTypedOptionalDefaultArg: mutation(
      (_ctx, { _arg }: { _arg?: string } = { _arg: "default" }) => "result",
    ),

    configNoArg: mutation({
      handler: () => "result",
    }),
    configValidatorNoArg: mutation({
      args: {},
      handler: () => "result",
    }),
    configUntypedArg: mutation({
      handler: (_, { arg }) => {
        assert<Equals<typeof arg, unknown>>;
        return "result";
      },
    }),
    configTypedArg: mutation({
      handler: (_, { arg }: { arg: string }) => {
        assert<Equals<typeof arg, string>>;
        return "result";
      },
    }),
    configOptionalValidatorUntypedArg: mutation({
      args: {
        arg: v.optional(v.string()),
      },
      handler: (_, { arg }) => {
        assert<Equals<typeof arg, string | undefined>>;
        return "result";
      },
    }),
    configValidatorUntypedArg: mutation({
      args: {
        arg: v.string(),
      },
      handler: (_, { arg }: { arg: string }) => {
        assert<Equals<typeof arg, string>>;
        return "result";
      },
    }),
    configValidatorTypedArg: mutation({
      args: {
        arg: v.string(),
      },
      handler: (_, { arg }: { arg: string }) => {
        assert<Equals<typeof arg, string>>;
        return "result";
      },
    }),
    configValidatorMismatchedTypedArg: mutation({
      args: {
        _arg: v.number(),
      },
      // @ts-expect-error  The arg type mismatches
      handler: (_, { _arg }: { _arg: string }) => {
        return "result";
      },
    }),
    configValidatorReturn: mutation({
      args: {
        _arg: v.number(),
      },
      returns: v.number(),
      // @ts-expect-error  The return type mismatches
      handler: (_, { _arg }) => {
        return "result";
      },
    }),

    // These are unusual syntaxes. We'd like to break some of them.
    // Let's document them here so it's clear when we do that.
    configUntypedDefaultArg: mutation({
      handler: (_, { arg } = { arg: "default" }) => {
        assert<Equals<typeof arg, unknown>>;
        return "result";
      },
    }),
    configTypedDefaultArg: mutation({
      // @ts-expect-error This syntax has never been allowed.
      handler: (_, { arg }: { arg: string } = { arg: "default" }) => {
        assert<Equals<typeof arg, string>>;
        return "result";
      },
    }),
    configTypedOptionalDefaultArg: mutation({
      // This syntax is incidentally allowed, it is not supported.
      handler: (_, { arg }: { arg?: string } = { arg: "default" }) => {
        assert<Equals<typeof arg, string | undefined>>;
        return "result";
      },
    }),
    configValidatorUntypedDefaultArg: mutation({
      args: {
        arg: v.string(),
      },
      handler: (_, { arg } = { arg: "default" }) => {
        assert<Equals<typeof arg, string>>;
        return "result";
      },
    }),
    configValidatorTypedDefaultArg: mutation({
      args: {
        arg: v.string(),
      },
      handler: (_, { arg }: { arg: string } = { arg: "default" }) => {
        assert<Equals<typeof arg, string>>;
        return "result";
      },
    }),
    configValidatorTypedOptionalDefaultArg: mutation({
      args: {
        arg: v.string(),
      },
      handler: (_, { arg }: { arg?: string } = { arg: "default" }) => {
        assert<Equals<typeof arg, string | undefined>>;
        return "result";
      },
    }),
  };
  type API = ApiFromModules<{ module: typeof module }>;

  test("inline with no arg", () => {
    type Args = API["module"]["inlineNoArg"]["_args"];
    assert<Equals<Args, EmptyObject>>();
    type ReturnType = API["module"]["inlineNoArg"]["_returnType"];
    assert<Equals<ReturnType, string>>();
  });

  test("inline with untyped arg", () => {
    type Args = API["module"]["inlineUntypedArg"]["_args"];
    type ExpectedArgs = DefaultFunctionArgs;
    assert<Equals<Args, ExpectedArgs>>;
  });

  test("inline with typed arg", () => {
    type Args = API["module"]["inlineTypedArg"]["_args"];
    type ExpectedArgs = { _arg: string };
    assert<Equals<Args, ExpectedArgs>>;
  });

  // This is not a very useful type (allows any key) but let's
  // test it so we know if it's changing.
  test("inline with untyped arg with default value", () => {
    type Args = API["module"]["inlineUntypedDefaultArg"]["_args"];
    type ExpectedArgs = DefaultFunctionArgs | EmptyObject;
    assert<Equals<Args, ExpectedArgs>>;
  });

  // This syntax is a type error where it is defined so it falls back.
  test("inline with typed arg with default value", () => {
    type Args = API["module"]["inlineTypedDefaultArg"]["_args"];
    type ExpectedArgs = Record<string, unknown>;
    assert<Equals<Args, ExpectedArgs>>;
  });

  // This is not a very useful type (allows any key) but add let's
  // test it so we know if it's changing.
  test("inline with typed arg with optional default value", () => {
    type Args = API["module"]["inlineTypedOptionalDefaultArg"]["_args"];
    type ExpectedArgs = DefaultFunctionArgs | EmptyObject;
    assert<Equals<Args, ExpectedArgs>>;
  });

  test("config with no arg", () => {
    type Args = API["module"]["configNoArg"]["_args"];
    type ExpectedArgs = EmptyObject;
    assert<Equals<Args, ExpectedArgs>>;
  });

  test("config with no arg and validator", () => {
    type Args = API["module"]["configValidatorNoArg"]["_args"];
    // eslint-disable-next-line @typescript-eslint/ban-types
    type ExpectedArgs = {};
    assert<Equals<Args, ExpectedArgs>>;
  });

  test("config with untyped arg", () => {
    type Args = API["module"]["configUntypedArg"]["_args"];
    type ExpectedArgs = DefaultFunctionArgs;
    assert<Equals<Args, ExpectedArgs>>;
  });

  test("config with typed arg", () => {
    type Args = API["module"]["configTypedArg"]["_args"];
    type ExpectedArgs = { arg: string };
    assert<Equals<Args, ExpectedArgs>>;
  });

  test("config with untyped arg and validator", () => {
    type Args = API["module"]["configValidatorUntypedArg"]["_args"];
    type ExpectedArgs = { arg: string };
    assert<Equals<Args, ExpectedArgs>>;
  });

  test("config with untyped arg and optional validator", () => {
    type Args = API["module"]["configOptionalValidatorUntypedArg"]["_args"];
    type ExpectedArgs = { arg?: string };
    assert<Equals<Args, ExpectedArgs>>;
  });

  test("config with typed arg and validator", () => {
    type Args = API["module"]["configValidatorTypedArg"]["_args"];
    type ExpectedArgs = { arg: string };
    assert<Equals<Args, ExpectedArgs>>;
  });

  test("config with untyped arg and a default", () => {
    type Args = API["module"]["configUntypedDefaultArg"]["_args"];
    // This is not a very useful type
    type ExpectedArgs = DefaultFunctionArgs | EmptyObject;
    assert<Equals<Args, ExpectedArgs>>;
  });

  test("config with typed arg and a default", () => {
    type Args = API["module"]["configTypedDefaultArg"]["_args"];
    // This is a type error at the definition site so this is the fallback.
    type ExpectedArgs = Record<string, unknown>;
    assert<Equals<Args, ExpectedArgs>>;
  });

  test("config with typed optional arg and a default", () => {
    type Args = API["module"]["configTypedOptionalDefaultArg"]["_args"];
    // This is not a very useful type
    type ExpectedArgs = DefaultFunctionArgs | EmptyObject;
    assert<Equals<Args, ExpectedArgs>>;
  });

  test("config with untyped arg and a validator and a default", () => {
    type Args = API["module"]["configValidatorUntypedDefaultArg"]["_args"];
    type ExpectedArgs = { arg: string };
    assert<Equals<Args, ExpectedArgs>>;
  });

  test("config with typed arg and a validator and a default", () => {
    type Args = API["module"]["configValidatorTypedDefaultArg"]["_args"];
    type ExpectedArgs = { arg: string };
    assert<Equals<Args, ExpectedArgs>>;
  });

  test("config with typed optional arg and a validator and a default", () => {
    type Args =
      API["module"]["configValidatorTypedOptionalDefaultArg"]["_args"];
    type ExpectedArgs = { arg: string };
    assert<Equals<Args, ExpectedArgs>>;
  });
});

describe("argument and return value validators can be objects or validators", () => {
  // Test with mutation, we aim for all the wrappers work the same way.
  const mutation: MutationBuilder<any, "public"> = mutationGeneric;
  const query: QueryBuilder<any, "public"> = queryGeneric;
  const action: ActionBuilder<any, "public"> = actionGeneric;

  const module = {
    configArgsObject: mutation({
      args: {
        arg: v.string(),
      },
      handler: (_, args) => {
        assert<Equals<(typeof args)["arg"], string>>;
        return "result";
      },
    }),
    configArgsValidatorIsNotSupported: mutation({
      args: v.object({
        arg: v.string(),
      }),
      handler: (_, args) => {
        assert<Equals<(typeof args)["arg"], string>>;
        return "result";
      },
    }),
    configOutputObject: mutation({
      returns: {
        arg: v.string(),
      },
      handler: () => {
        return { arg: "result" };
      },
    }),
    configOutputValidator: mutation({
      returns: v.object({
        arg: v.string(),
      }),
      handler: () => {
        return { arg: "result" };
      },
    }),

    // test queries and actions just a bit too
    q1: query({
      args: v.object({
        arg: v.string(),
      }),
      returns: { arg: v.string() },
      handler: (_, { arg }) => {
        return { arg: arg };
      },
    }),

    a1: action({
      args: v.object({
        arg: v.string(),
      }),
      returns: { arg: v.string() },
      handler: (_, { arg }) => {
        return { arg: arg };
      },
    }),
  };
  type API = ApiFromModules<{ module: typeof module }>;

  const expectedArgsExport = {
    type: "object",
    value: {
      arg: {
        fieldType: {
          type: "string",
        },
        optional: false,
      },
    },
  };

  const expectedReturnsExport = {
    type: "object",
    value: {
      arg: {
        fieldType: {
          type: "string",
        },
        optional: false,
      },
    },
  };

  test("config with args validator", () => {
    type Args = API["module"]["configArgsObject"]["_args"];
    type ExpectedArgs = { arg: string };
    assert<Equals<Args, ExpectedArgs>>;
    const argsString = module.configArgsObject.exportArgs();
    expect(JSON.parse(argsString)).toEqual(expectedArgsExport);
  });

  test("config with args object", () => {
    type Args = API["module"]["configArgsValidatorIsNotSupported"]["_args"];
    type ExpectedArgs = { arg: string };
    assert<Equals<Args, ExpectedArgs>>;
    const argsString = module.configArgsObject.exportArgs();
    expect(JSON.parse(argsString)).toEqual(expectedArgsExport);
  });

  test("config with output validator", () => {
    type ReturnType = API["module"]["configOutputObject"]["_returnType"];
    type Expected = { arg: string };
    assert<Equals<ReturnType, Expected>>;
    const returnString = module.configOutputObject.exportReturns();
    expect(JSON.parse(returnString)).toEqual(expectedReturnsExport);
  });

  test("config with output object", () => {
    type ReturnType = API["module"]["configOutputValidator"]["_returnType"];
    type Expected = { arg: string };
    assert<Equals<ReturnType, Expected>>;
    const returnString = module.configOutputValidator.exportReturns();
    expect(JSON.parse(returnString)).toEqual(expectedReturnsExport);
  });

  test("queries", () => {
    type ReturnType = API["module"]["q1"]["_returnType"];
    type Expected = { arg: string };
    assert<Equals<ReturnType, Expected>>;
    const returnString = module.configOutputValidator.exportReturns();
    expect(JSON.parse(returnString)).toEqual(expectedReturnsExport);
  });

  test("actions", () => {
    type ReturnType = API["module"]["configOutputValidator"]["_returnType"];
    type Expected = { arg: string };
    assert<Equals<ReturnType, Expected>>;
    const returnString = module.configOutputValidator.exportReturns();
    expect(JSON.parse(returnString)).toEqual(expectedReturnsExport);
  });
});
