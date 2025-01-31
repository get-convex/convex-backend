import {
  FunctionExecution,
  FunctionExecutionCompletion,
  FunctionExecutionProgess,
} from "system-udfs/convex/_system/frontend/common";
import { entryOutcome, entryOutput, processLogs } from "lib/useLogs";
import { functionIdentifierValue } from "lib/functions/generateFileTree";
import { formatDateTime } from "lib/format";

function createExecutionCompletion(
  overrides: Partial<FunctionExecutionCompletion>,
): FunctionExecutionCompletion {
  return {
    kind: "Completion",
    requestId: "myreqeustid",
    executionId: "myexecutionid",
    logLines: [
      {
        level: "INFO",
        isTruncated: false,
        timestamp: 900,
        messages: ["Hello [Hello] Hello"],
      },
    ],
    identifier: "myFunctions:doSomething",
    udfType: "Mutation",
    arguments: [],
    timestamp: 1,
    cachedResult: false,
    executionTime: 1,
    error: null,
    success: null,
    ...overrides,
  };
}

function createExecutionProgress(
  overrides: Partial<FunctionExecutionProgess>,
): FunctionExecutionProgess {
  return {
    kind: "Progress",
    requestId: "myreqeustid",
    executionId: "myexecutionid",
    logLines: [
      {
        level: "INFO",
        isTruncated: false,
        timestamp: 900,
        messages: ["Hello [Hello] Hello"],
      },
    ],
    identifier: "myFunctions:doSomething",
    udfType: "Mutation",
    timestamp: 1,
    ...overrides,
  };
}

describe("entryOutput", () => {
  it("should not truncate text that contains extra brackets", () => {
    expect(
      entryOutput({
        logLines: ["[INFO] Hello [Hello] Hello"],
        error: null,
      }),
    ).toEqual([
      {
        level: "INFO",
        messages: ["Hello [Hello] Hello"],
        isTruncated: false,
        isUnstructured: true,
      },
    ]);
  });
  // '\n' characters in developer console.log('a\nb') strings are escaped
  // (a behavior we should probably fix) but unescaped newline characters
  // do appear in large pretty-print outputs.
  it("should not truncate multiline text", () => {
    expect(
      entryOutput({
        logLines: ["[INFO] Hello\nHello"],
        error: null,
      }),
    ).toEqual([
      {
        level: "INFO",
        messages: ["Hello\nHello"],
        isTruncated: false,
        isUnstructured: true,
      },
    ]);
  });
});

describe("processLogs", () => {
  it("should process raw logs and format them correctly", () => {
    const rawLogs = [createExecutionCompletion({ error: "Whoopsie" })];

    const expectedLogs = [
      {
        kind: "log",
        udfType: "Mutation",
        id: "1",
        call: functionIdentifierValue("myFunctions.js:doSomething"),
        requestId: "myreqeustid",
        executionId: "myexecutionid",
        localizedTimestamp: formatDateTime(new Date(900)),
        timestamp: 900,
        output: {
          level: "INFO",
          messages: ["Hello [Hello] Hello"],
          isTruncated: false,
          timestamp: 900,
        },
      },
      {
        udfType: "Mutation",
        id: "2",
        localizedTimestamp: formatDateTime(new Date(1000)),
        timestamp: 1000,
        error: "Whoopsie",
        call: functionIdentifierValue("myFunctions.js:doSomething"),
        cachedResult: false,
        requestId: "myreqeustid",
        executionId: "myexecutionid",
        outcome: {
          status: "failure",
          statusCode: null,
        },
        executionTimeMs: rawLogs[0].executionTime * 1000,
        kind: "outcome",
      },
    ];

    const processedLogs = processLogs(rawLogs);

    expect(processedLogs).toEqual(expectedLogs);
  });

  it("should handle progress logs", () => {
    const rawLogs = [createExecutionProgress({})];

    const expectedLogs = [
      {
        kind: "log",
        udfType: "Mutation",
        id: "3",
        call: functionIdentifierValue("myFunctions.js:doSomething"),
        requestId: "myreqeustid",
        executionId: "myexecutionid",
        localizedTimestamp: formatDateTime(new Date(900)),
        timestamp: 900,
        output: {
          level: "INFO",
          messages: ["Hello [Hello] Hello"],
          isTruncated: false,
          timestamp: 900,
          subfunction: undefined,
        },
      },
    ];

    const processedLogs = processLogs(rawLogs);

    expect(processedLogs).toEqual(expectedLogs);
  });

  it("should handle progress with componentPath=null", () => {
    const rawLogs = [createExecutionProgress({ componentPath: null })];
    const expectedLogs = [
      {
        kind: "log",
        udfType: "Mutation",
        id: "4",
        call: functionIdentifierValue("myFunctions.js:doSomething"),
        requestId: "myreqeustid",
        executionId: "myexecutionid",
        localizedTimestamp: formatDateTime(new Date(900)),
        timestamp: 900,
        output: {
          level: "INFO",
          messages: ["Hello [Hello] Hello"],
          isTruncated: false,
          timestamp: 900,
          subfunction: undefined,
        },
      },
    ];

    const processedLogs = processLogs(rawLogs);

    expect(processedLogs).toEqual(expectedLogs);
  });

  it("should handle empty raw logs", () => {
    const rawLogs: FunctionExecution[] = [];

    const processedLogs = processLogs(rawLogs);

    expect(processedLogs).toEqual([]);
  });
});

describe("entryOutcome", () => {
  it("with an error string it should be an error", () => {
    expect(
      entryOutcome(
        createExecutionCompletion({
          error: "Blah",
        }),
      ),
    ).toEqual({ status: "failure", statusCode: null });
  });

  // This should never actually happen because udf actions should not have
  // status codes, but we can try.
  it("with a success value for actions, it should be a success", () => {
    expect(
      entryOutcome(
        createExecutionCompletion({
          error: null,
          success: { status: "200" },
        }),
      ),
    ).toEqual({ status: "success", statusCode: null });
  });

  it("with a success value for http actions, it should be a success", () => {
    expect(
      entryOutcome(
        createExecutionCompletion({
          udfType: "HttpAction",
          error: null,
          success: { status: "200" },
        }),
      ),
    ).toEqual({ status: "success", statusCode: "200" });
  });

  it("with a null error and success values for udf actions it should be a success", () => {
    expect(
      entryOutcome(
        createExecutionCompletion({
          udfType: "Action",
          success: null,
          error: null,
        }),
      ),
    ).toEqual({ status: "success", statusCode: null });
  });

  it("with a null error and success values for http actions it should be an error", () => {
    expect(
      entryOutcome(
        createExecutionCompletion({
          udfType: "HttpAction",
          error: null,
          success: null,
        }),
      ),
    ).toEqual({ status: "failure", statusCode: null });
  });

  it("with an undefined error and a 0 status code for http actions it should be an error", () => {
    expect(
      entryOutcome(
        createExecutionCompletion({
          udfType: "HttpAction",
          error: undefined,
          success: { status: "0" },
        }),
      ),
    ).toEqual({ status: "failure", statusCode: "0" });
  });

  it("with an undefined error and a 200 status code for http actions it should be a success", () => {
    expect(
      entryOutcome(
        createExecutionCompletion({
          udfType: "HttpAction",
          error: undefined,
          success: { status: "200" },
        }),
      ),
    ).toEqual({ status: "success", statusCode: "200" });
  });

  it("for http actions it should use successful http status code", () => {
    expect(
      entryOutcome(
        createExecutionCompletion({
          udfType: "HttpAction",
          error: null,
          success: { status: "201" },
        }),
      ),
    ).toEqual({ status: "success", statusCode: "201" });
  });

  // This case seems quite weird, it's unclear why we'd expect to get an invalid status code in
  // 'success'
  it("for http actions it should use unsuccessful http status code", () => {
    expect(
      entryOutcome(
        createExecutionCompletion({
          udfType: "HttpAction",
          error: null,
          success: { status: "404" },
        }),
      ),
    ).toEqual({ status: "failure", statusCode: "404" });
  });

  it("should use 500 HTTP status code on error", () => {
    expect(
      entryOutcome(
        createExecutionCompletion({
          udfType: "HttpAction",
          error: "Oh no!",
          success: null,
        }),
      ),
    ).toEqual({ status: "failure", statusCode: "500" });
  });
});
