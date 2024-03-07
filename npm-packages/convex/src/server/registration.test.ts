import { test } from "@jest/globals";
import { assert, Equals } from "../test/type_testing.js";
import { v } from "../values/validator.js";
import { ApiFromModules, DefaultFunctionArgs } from "./index.js";
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
    // This error could be prettier if we stop overloading the builders.
    // @ts-expect-error  The arg type mismatches
    configValidatorMismatchedTypedArg: mutation({
      args: {
        _arg: v.number(),
      },
      handler: (_, { _arg }: { _arg: string }) => {
        return "result";
      },
    }),
  };
  type API = ApiFromModules<{ module: typeof module }>;

  test("inline with no arg", () => {
    type Args = API["module"]["inlineNoArg"]["_args"];
    assert<Equals<Args, EmptyObject>>();
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
});
