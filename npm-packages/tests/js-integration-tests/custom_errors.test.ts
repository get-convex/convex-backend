import { ConvexClient, ConvexHttpClient } from "convex/browser";
import { ConvexReactClient } from "convex/react";
import { ConvexError } from "convex/values";
import { api } from "./convex/_generated/api";
import { opts } from "./test_helpers";
import { FunctionReference } from "convex/server";
import { deploymentUrl, siteUrl } from "./common";

describe("HTTP API", () => {
  test("http action throwing ConvexError", async () => {
    const url = `${siteUrl}/failer_custom`;
    const response = await fetch(url);
    expect(response.ok).toEqual(false);
    const result = await response.json();
    expect(result.code).toMatch(
      /\[Request ID: [a-f0-9]{16}\] Server Error: Uncaught ConvexError: Hello world!/,
    );
    expect(result.data).toEqual("Hello world!");
  });
});

type TestCase = {
  name: string;
  call: () => Promise<unknown>;
  errorExpectation: (error: unknown) => void;
};

describe("ConvexHttpClient", () => {
  let httpClient: ConvexHttpClient;

  beforeEach(() => {
    httpClient = new ConvexHttpClient(deploymentUrl);
  });
  afterEach(async () => {
    await httpClient.mutation(api.cleanUp.default);
  });

  test.each<TestCase>([...sharedTestCases(() => httpClient)])(
    `from $name`,
    async ({ call, errorExpectation }) => {
      let error: unknown;
      try {
        await call();
      } catch (e) {
        error = e;
      }
      errorExpectation(error);
    },
  );
});

describe("ConvexReactClient", () => {
  let reactClient: ConvexReactClient;
  beforeEach(() => {
    reactClient = new ConvexReactClient(deploymentUrl, opts);
  });
  afterEach(async () => {
    await reactClient.mutation(api.cleanUp.default);
    await reactClient.close();
  });

  test.each<TestCase>([...sharedTestCases(() => reactClient)])(
    `from $name`,
    async ({ call, errorExpectation }) => {
      let error: unknown;
      try {
        await call();
      } catch (e) {
        error = e;
      }
      errorExpectation(error);
    },
  );
});

describe("ConvexClient", () => {
  let simpleClient: ConvexClient;
  beforeEach(() => {
    simpleClient = new ConvexClient(deploymentUrl, opts);
  });
  afterEach(async () => {
    await simpleClient.mutation(api.cleanUp.default, {});
    await simpleClient.close();
  });

  test.each<TestCase>([...sharedTestCases(() => simpleClient)])(
    `from $name`,
    async ({ call, errorExpectation }) => {
      let error: unknown;
      try {
        await call();
      } catch (e) {
        error = e;
      }
      errorExpectation(error);
    },
  );

  test("Scheduler", async () => {
    await simpleClient.mutation(
      api.customErrors.mutationSchedulingActionCallingMutation,
      {},
    );

    // Wait long enough for the scheduled call to happen
    await new Promise((resolve) => setTimeout(resolve, 2000));

    const users = await simpleClient.query(api.getUsers.default, {});
    expect(users[0]?.name).toEqual("ConvexError");
  });
});

function sharedTestCases(
  client: () => {
    query(fn: FunctionReference<"query">, args?: any): Promise<unknown>;
    mutation(fn: FunctionReference<"mutation">, args?: any): Promise<unknown>;
    action(fn: FunctionReference<"action">, args?: any): Promise<unknown>;
  },
): TestCase[] {
  return [
    {
      name: "query throwing ConvexError",
      call: () => client().query(api.customErrors.queryThrowingConvexError),
      errorExpectation: expectSimpleError(
        `Uncaught ConvexError: Boom boom bop
    at <anonymous> (../convex/customErrors.ts:NUM:NUM)`,
      ),
    },
    {
      name: "query throwing normal Error",
      call: () => client().query(api.customErrors.queryThrowingNormalError),
      errorExpectation: (error) => {
        expect(error).not.toBeInstanceOf(ConvexError);
        expect(error).toBeInstanceOf(Error);
        if (error instanceof Error) {
          expect(error.message.trim()).toEqual(
            matchingErrorMessage(`Uncaught Error: Normal error
    at <anonymous> (../convex/customErrors.ts:NUM:NUM)`),
          );
          expect(error.name).toEqual("Error");
          expect((error as any).data).toBeUndefined();
        }
      },
    },
    {
      name: "query throwing ConvexError subclass",
      call: () =>
        client().query(api.customErrors.queryThrowingConvexErrorSubclass),
      errorExpectation: expectErrorWithCode(
        `Uncaught FooError: {"message":"Boom boom bop","code":"123n"}
    at <anonymous> (../convex/customErrors.ts:NUM:NUM)`,
      ),
    },
    {
      name: "mutation throwing ConvexError",
      call: () =>
        client().mutation(api.customErrors.mutationThrowingConvexError),
      errorExpectation: expectErrorWithCode(
        `Uncaught ConvexError: {"message":"Boom boom bop","code":"123n"}
    at <anonymous> (../convex/customErrors.ts:NUM:NUM)`,
      ),
    },
    {
      name: "action throwing ConvexError",
      call: () => client().action(api.customErrors.actionThrowingConvexError),
      errorExpectation: expectSimpleError(
        `Uncaught ConvexError: Boom boom bop
    at <anonymous> (../convex/customErrors.ts:NUM:NUM)`,
      ),
    },
    {
      name: "node action throwing ConvexError",
      call: () =>
        client().action(
          api.customErrorsNodeActions.nodeActionThrowingConvexError,
        ),
      errorExpectation: expectSimpleError(
        `Uncaught ConvexError: Boom boom bop
    at <anonymous> (../convex/customErrorsNodeActions.ts:NUM:NUM)`,
      ),
    },
    {
      name: "action calling query throwing ConvexError",
      call: () =>
        client().action(api.customErrors.actionCallingQueryThrowingConvexError),
      errorExpectation: expectSimpleError(
        // TODO: Fix the error message double nesting
        `Uncaught ConvexError: Uncaught ConvexError: Boom boom bop
    at <anonymous> (../convex/customErrors.ts:NUM:NUM)

    at async <anonymous> (../convex/customErrors.ts:NUM:NUM)`,
      ),
    },
    {
      name: "action calling query throwing ConvexError subclass",
      call: () =>
        client().action(
          api.customErrors.actionCallingQueryThrowingConvexErrorSubclass,
        ),
      errorExpectation: expectErrorWithCode(
        `Uncaught ConvexError: Uncaught FooError: {"message":"Boom boom bop","code":"123n"}
    at <anonymous> (../convex/customErrors.ts:NUM:NUM)

    at async <anonymous> (../convex/customErrors.ts:NUM:NUM)`,
      ),
    },
    {
      name: "action calling mutation throwing ConvexError",
      call: () =>
        client().action(
          api.customErrors.actionCallingMutationThrowingConvexError,
        ),
      errorExpectation: expectErrorWithCode(
        `Uncaught ConvexError: Uncaught ConvexError: {"message":"Boom boom bop","code":"123n"}
    at <anonymous> (../convex/customErrors.ts:NUM:NUM)

    at async <anonymous> (../convex/customErrors.ts:NUM:NUM)`,
      ),
    },
    {
      name: "v8 action calling action throwing ConvexError",
      call: () =>
        client().action(
          api.customErrors.actionCallingActionThrowingConvexError,
        ),
      errorExpectation: expectSimpleError(
        `Uncaught ConvexError: Uncaught ConvexError: Boom boom bop
    at <anonymous> (../convex/customErrors.ts:NUM:NUM)

    at async <anonymous> (../convex/customErrors.ts:NUM:NUM)`,
      ),
    },
    {
      name: "v8 action calling node action throwing ConvexError",
      call: () =>
        client().action(
          api.customErrors.actionCallingNodeActionThrowingConvexError,
        ),
      errorExpectation: expectSimpleError(
        `Uncaught ConvexError: Uncaught ConvexError: Boom boom bop
    at <anonymous> (../convex/customErrorsNodeActions.ts:NUM:NUM)

    at async <anonymous> (../convex/customErrors.ts:NUM:NUM)`,
      ),
    },
    {
      name: "node action calling query throwing ConvexError",
      call: () =>
        client().action(
          api.customErrorsNodeActions.nodeActionCallingQueryThrowingConvexError,
        ),
      errorExpectation: expectSimpleError(
        `Uncaught ConvexError: Uncaught ConvexError: Boom boom bop
    at <anonymous> (../convex/customErrors.ts:NUM:NUM)

    at async <anonymous> (../convex/customErrorsNodeActions.ts:NUM:NUM)`,
      ),
    },
    {
      name: "node action calling query throwing ConvexError subclass",
      call: () =>
        client().action(
          api.customErrorsNodeActions
            .nodeActionCallingQueryThrowingConvexErrorSubclass,
        ),
      errorExpectation: expectErrorWithCode(
        `Uncaught ConvexError: Uncaught FooError: {"message":"Boom boom bop","code":"123n"}
    at <anonymous> (../convex/customErrors.ts:NUM:NUM)

    at async <anonymous> (../convex/customErrorsNodeActions.ts:NUM:NUM)`,
      ),
    },
    {
      name: "node action calling mutation throwing ConvexError",
      call: () =>
        client().action(
          api.customErrorsNodeActions
            .nodeActionCallingMutationThrowingConvexError,
        ),
      errorExpectation: expectErrorWithCode(
        `Uncaught ConvexError: Uncaught ConvexError: {"message":"Boom boom bop","code":"123n"}
    at <anonymous> (../convex/customErrors.ts:NUM:NUM)

    at async <anonymous> (../convex/customErrorsNodeActions.ts:NUM:NUM)`,
      ),
    },
    {
      name: "node action calling v8 action throwing ConvexError",
      call: () =>
        client().action(
          api.customErrorsNodeActions
            .nodeActionCallingActionThrowingConvexError,
        ),
      errorExpectation: expectSimpleError(
        `Uncaught ConvexError: Uncaught ConvexError: Boom boom bop
    at <anonymous> (../convex/customErrors.ts:NUM:NUM)

    at async <anonymous> (../convex/customErrorsNodeActions.ts:NUM:NUM)`,
      ),
    },
    {
      name: "node action calling node action throwing ConvexError",
      call: () =>
        client().action(
          api.customErrorsNodeActions
            .nodeActionCallingNodeActionThrowingConvexError,
        ),
      errorExpectation: expectSimpleError(
        `Uncaught ConvexError: Uncaught ConvexError: Boom boom bop
    at <anonymous> (../convex/customErrorsNodeActions.ts:NUM:NUM)

    at async <anonymous> (../convex/customErrorsNodeActions.ts:NUM:NUM)`,
      ),
    },
    {
      name: "query calling component query throwing ConvexError",
      call: () =>
        client().query(api.customErrors.componentQueryThrowingConvexError),
      errorExpectation: expectSimpleError(
        `Uncaught ConvexError: Uncaught ConvexError: Boom boom bop
    at <anonymous> (../component/errors.ts:NUM:NUM)

    at async <anonymous> (../convex/customErrors.ts:NUM:NUM)`,
      ),
    },
    {
      name: "query calling component throwing normal Error",
      call: () => client().query(api.customErrors.componentQueryThrowingError),
      errorExpectation: (error) => {
        expect(error).not.toBeInstanceOf(ConvexError);
        expect(error).toBeInstanceOf(Error);
        if (error instanceof Error) {
          expect(error.message.trim()).toEqual(
            matchingErrorMessage(`Uncaught Error: Uncaught Error: component kaboom
    at <anonymous> (../component/errors.ts:NUM:NUM)

    at async <anonymous> (../convex/customErrors.ts:NUM:NUM)`),
          );
          expect(error.name).toEqual("Error");
          expect((error as any).data).toBeUndefined();
        }
      },
    },
    {
      name: "action calling component query throwing ConvexError",
      call: () =>
        client().action(
          api.customErrors.actionCallingComponentQueryThrowingConvexError,
        ),
      errorExpectation: expectSimpleError(
        `Uncaught ConvexError: Uncaught ConvexError: Boom boom bop
    at <anonymous> (../component/errors.ts:NUM:NUM)

    at async <anonymous> (../convex/customErrors.ts:NUM:NUM)`,
      ),
    },
  ];
}

function expectSimpleError(errorMessageRegexSource: string) {
  return (error: unknown) => {
    expect(error).toBeInstanceOf(ConvexError);
    if (error instanceof ConvexError) {
      expect(error.message.trim()).toEqual(
        matchingErrorMessage(errorMessageRegexSource),
      );
      expect(error.name).toEqual("ConvexError");
      expect(error.data).toEqual("Boom boom bop");
    }
  };
}

function expectErrorWithCode(errorMessageRegexSource: string) {
  return (error: unknown) => {
    expect(error).toBeInstanceOf(ConvexError);
    if (error instanceof ConvexError) {
      expect(error.message.trim()).toEqual(
        matchingErrorMessage(errorMessageRegexSource),
      );
      expect(error.name).toEqual("ConvexError");
      expect(error.data.message).toEqual("Boom boom bop");
      expect(error.data.code).toEqual(123n);
    }
  };
}

function matchingErrorMessage(errorMessageRegexSource: string) {
  return expect.stringMatching(
    new RegExp(
      "^" +
        // React error prefix
        "(\\[CONVEX [QMA]\\([a-zA-Z:]+\\)\\] )?" +
        "\\[Request ID: [a-f0-9]{16}\\] Server Error\n" +
        errorMessageRegexSource
          .trim()
          .replace(/\./g, "\\.")
          .replace(/\(/g, "\\(")
          .replace(/\)/g, "\\)")
          .replace(/NUM/g, "\\d+") +
        // React error suffix
        "(\n\n  Called by client)?" +
        "$",
    ),
  );
}
