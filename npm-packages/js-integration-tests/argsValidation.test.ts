import { ConvexHttpClient } from "convex/browser";
import { ConvexReactClient } from "convex/react";
import { api } from "./convex/_generated/api";
import { opts } from "./test_helpers";
import { deploymentUrl } from "./common";

describe("HTTPClient argument validation", () => {
  let httpClient: ConvexHttpClient;

  beforeEach(() => {
    httpClient = new ConvexHttpClient(deploymentUrl);
  });

  afterEach(async () => {
    await httpClient.mutation(api.cleanUp.default);
  });

  test("query with wrong argument type (number instead of string)", async () => {
    await expect(
      httpClient.query(api.argsValidation.queryWithStringArg, {
        name: 123 as any,
      }),
    ).rejects.toThrow("ArgumentValidationError");
  });

  test("query with wrong argument type (string instead of number)", async () => {
    await expect(
      httpClient.query(api.argsValidation.queryWithNumberArg, {
        count: "not a number" as any,
      }),
    ).rejects.toThrow("ArgumentValidationError");
  });

  test("query with missing required argument", async () => {
    await expect(
      httpClient.query(api.argsValidation.queryWithStringArg, {} as any),
    ).rejects.toThrow("ArgumentValidationError");
  });

  test("query with extra unexpected argument", async () => {
    await expect(
      httpClient.query(api.argsValidation.queryWithStringArg, {
        name: "test",
        extraArg: "unexpected",
      } as any),
    ).rejects.toThrow("ArgumentValidationError");
  });

  test("query with invalid object structure", async () => {
    await expect(
      httpClient.query(api.argsValidation.queryWithObjectArg, {
        user: { name: "test" } as any, // missing age
      }),
    ).rejects.toThrow("ArgumentValidationError");
  });

  test("query with wrong type in nested object", async () => {
    await expect(
      httpClient.query(api.argsValidation.queryWithObjectArg, {
        user: { name: 123, age: 25 } as any, // name should be string
      }),
    ).rejects.toThrow("ArgumentValidationError");
  });

  test("mutation with missing required arguments", async () => {
    await expect(
      httpClient.mutation(api.argsValidation.mutationWithRequiredArgs, {
        channel: "test",
        // missing 'text'
      } as any),
    ).rejects.toThrow("ArgumentValidationError");
  });

  test("action with wrong argument types", async () => {
    await expect(
      httpClient.action(api.argsValidation.actionWithValidation, {
        email: 123 as any,
        count: "not a number" as any,
      }),
    ).rejects.toThrow("ArgumentValidationError");
  });

  test("query with multiple args where one is wrong", async () => {
    await expect(
      httpClient.query(api.argsValidation.queryWithMultipleArgs, {
        required: "test",
        number: "not a number" as any,
      }),
    ).rejects.toThrow("ArgumentValidationError");
  });

  test("query with union type receiving invalid type", async () => {
    await expect(
      httpClient.query(api.argsValidation.queryWithUnion, {
        value: true as any, // boolean not in union(string, number)
      }),
    ).rejects.toThrow("ArgumentValidationError");
  });

  test("query with array type receiving non-array", async () => {
    await expect(
      httpClient.query(api.argsValidation.queryWithArray, {
        items: "not an array" as any,
      }),
    ).rejects.toThrow("ArgumentValidationError");
  });

  test("query with array containing wrong types", async () => {
    await expect(
      httpClient.query(api.argsValidation.queryWithArray, {
        items: [1, 2, 3] as any, // should be strings
      }),
    ).rejects.toThrow("ArgumentValidationError");
  });

  test("valid arguments should succeed", async () => {
    const result = await httpClient.query(
      api.argsValidation.queryWithStringArg,
      {
        name: "Alice",
      },
    );
    expect(result).toBe("Hello, Alice!");
  });

  test("valid object arguments should succeed", async () => {
    const result = await httpClient.query(
      api.argsValidation.queryWithObjectArg,
      {
        user: { name: "Bob", age: 30 },
      },
    );
    expect(result).toEqual({ name: "Bob", age: 30 });
  });

  test("valid union arguments should succeed", async () => {
    const result1 = await httpClient.query(api.argsValidation.queryWithUnion, {
      value: "test",
    });
    expect(result1).toBe("test");

    const result2 = await httpClient.query(api.argsValidation.queryWithUnion, {
      value: 42,
    });
    expect(result2).toBe(42);
  });
});

describe("ConvexReactClient argument validation", () => {
  let reactClient: ConvexReactClient;

  beforeEach(() => {
    reactClient = new ConvexReactClient(deploymentUrl, opts);
  });

  afterEach(async () => {
    await reactClient.mutation(api.cleanUp.default);
    await reactClient.close();
  });

  test("query with wrong argument type", async () => {
    await expect(
      reactClient.query(api.argsValidation.queryWithStringArg, {
        name: 123 as any,
      }),
    ).rejects.toThrow("ArgumentValidationError");
  });

  test("mutation with missing required arguments", async () => {
    await expect(
      reactClient.mutation(api.argsValidation.mutationWithRequiredArgs, {
        channel: "test",
      } as any),
    ).rejects.toThrow("ArgumentValidationError");
  });

  test("action with wrong argument types", async () => {
    await expect(
      reactClient.action(api.argsValidation.actionWithValidation, {
        email: 123 as any,
        count: "not a number" as any,
      }),
    ).rejects.toThrow("ArgumentValidationError");
  });

  test("valid arguments should succeed", async () => {
    const result = await reactClient.query(
      api.argsValidation.queryWithStringArg,
      {
        name: "Charlie",
      },
    );
    expect(result).toBe("Hello, Charlie!");
  });
});
