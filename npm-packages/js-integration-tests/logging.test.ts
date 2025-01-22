import { ConvexHttpClient } from "convex/browser";
import { api } from "./convex/_generated/api";
import fs from "fs";
import { deploymentUrl } from "./common";

type TestCase = {
  name: string;
  call: () => Promise<unknown>;
};

describe("ConvexHttpClient", () => {
  let httpClient: ConvexHttpClient;

  const adminKey = fs.readFileSync(
    "../../crates/keybroker/dev/admin_key.txt",
    "utf8",
  );

  beforeEach(() => {
    httpClient = new ConvexHttpClient(deploymentUrl);
    httpClient.setAdminAuth(adminKey);
  });

  test.each<TestCase>(testCases())(`$name`, async ({ call }) => {
    const logSpy = jest.spyOn(console, "log");

    try {
      await call();
    } catch (error: any) {
      expect(error.message).toEqual(expect.stringContaining("oopsie"));
    }

    expect(logSpy).toHaveBeenCalledWith(
      expect.stringContaining("[LOG]"),
      expect.stringContaining("color"),
      expect.stringContaining("Important logged stuff"),
    );

    logSpy.mockClear();
  });

  function testCases(): TestCase[] {
    return [
      {
        name: "query succeeding and logging",
        call: () => httpClient.query(api.logging.queryLogging),
      },
      {
        name: "query throwing and logging",
        call: () => httpClient.query(api.logging.queryLoggingAndThrowing),
      },
      {
        name: "mutation succeeding and logging",
        call: () => httpClient.mutation(api.logging.mutationLogging),
      },
      {
        name: "mutation throwing and logging",
        call: () => httpClient.mutation(api.logging.mutationLoggingAndThrowing),
      },
      {
        name: "action succeeding and logging",
        call: () => httpClient.action(api.logging.actionLogging),
      },
      {
        name: "action throwing and logging",
        call: () => httpClient.action(api.logging.actionLoggingAndThrowing),
      },
      {
        name: "query succeeding and logging via function",
        call: () => httpClient.function(api.logging.queryLogging),
      },
      {
        name: "query throwing and logging via function",
        call: () => httpClient.function(api.logging.queryLoggingAndThrowing),
      },
      {
        name: "mutation succeeding and logging via function",
        call: () => httpClient.function(api.logging.mutationLogging),
      },
      {
        name: "mutation throwing and logging via function",
        call: () => httpClient.function(api.logging.mutationLoggingAndThrowing),
      },
      {
        name: "action succeeding and logging via function",
        call: () => httpClient.function(api.logging.actionLogging),
      },
      {
        name: "action throwing and logging via function",
        call: () => httpClient.function(api.logging.actionLoggingAndThrowing),
      },
    ];
  }

  test("node action consoleTime", async () => {
    const logSpy = jest.spyOn(console, "log");
    const messages: string[] = [];
    logSpy.mockImplementation((_prefix, _color, message) =>
      messages.push(message),
    );
    await httpClient.action(api.actions.simple.consoleTime);
    expect(messages.length).toEqual(6);
    expect(messages[0]).toMatch(/Log at import time/);
    expect(messages[1]).toMatch(/default: [0-9]+ms/);
    expect(messages[2]).toMatch(/default: [0-9]+ms/);
    expect(messages[3]).toMatch(/Timer 'foo' already exists/);
    expect(messages[4]).toMatch(/'foo: [0-9]+ms' 'bar' 'baz'/);
    expect(messages[5]).toMatch(/foo: [0-9]+ms/);
  });
});
