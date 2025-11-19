import omit from "lodash/omit";
import { filterLogs, ALL_LEVELS } from "@common/features/logs/lib/filterLogs";
import { functionIdentifierValue } from "@common/lib/functions/generateFileTree";
import { UdfLog } from "@common/lib/useLogs";
import { NENT_APP_PLACEHOLDER } from "@common/lib/useNents";

const logs: UdfLog[] = [
  {
    id: "1",
    kind: "log",
    timestamp: 1,
    localizedTimestamp: "2022-06-01T23:18:59.467Z",
    udfType: "Mutation",
    call: functionIdentifierValue("mutateData"),
    output: { level: "DEBUG", messages: ["Log!"], isTruncated: false },
    requestId: "first",
    executionId: "1",
  },
  {
    id: "2",
    kind: "outcome",
    timestamp: 1,
    outcome: { status: "success", statusCode: null },
    localizedTimestamp: "2022-06-01T23:18:59.467Z",
    udfType: "Mutation",
    call: functionIdentifierValue("mutateData"),
    executionTimeMs: 30,
    requestId: "first",
    executionId: "1",
    caller: "test",
    environment: "isolate",
    identityType: "user",
    parentExecutionId: null,
  },
  {
    id: "3",
    kind: "log",
    timestamp: 1,
    localizedTimestamp: "2022-06-01T23:18:59.467Z",
    udfType: "Query",
    call: functionIdentifierValue("queryData"),
    output: {
      level: "INFO",
      messages: ["Another result!"],
      isTruncated: false,
    },
    requestId: "second",
    executionId: "2",
  },
  {
    id: "4",
    kind: "log",
    timestamp: 1,
    localizedTimestamp: "2022-06-01T23:18:59.467Z",
    udfType: "Query",
    call: functionIdentifierValue("queryData"),
    output: {
      level: "INFO",
      messages: ["Log from subfunction"],
      isTruncated: false,
      subfunction: functionIdentifierValue("subquery"),
    },
    requestId: "second",
    executionId: "2",
  },
  {
    id: "5",
    kind: "outcome",
    outcome: { status: "success", statusCode: null },
    executionTimeMs: 30,
    timestamp: 1,
    localizedTimestamp: "2022-06-01T23:18:59.467Z",
    udfType: "Query",
    call: functionIdentifierValue("queryData"),
    requestId: "second",
    executionId: "2",
    caller: "test",
    environment: "isolate",
    identityType: "user",
    parentExecutionId: null,
  },
  {
    id: "6",
    kind: "log",
    localizedTimestamp: "2022-06-01T23:18:59.467Z",
    timestamp: Date.parse("2022-06-01T23:18:59.467Z"),
    udfType: "Query",
    call: functionIdentifierValue("queryData"),
    output: { level: "ERROR", messages: ["Log!"], isTruncated: false },
    requestId: "third",
    executionId: "3",
  },
  {
    id: "7",
    kind: "outcome",
    localizedTimestamp: "2022-06-01T23:18:59.467Z",
    timestamp: Date.parse("2022-06-01T23:18:59.467Z"),
    outcome: { status: "success", statusCode: null },
    udfType: "Query",
    call: functionIdentifierValue("queryData"),
    executionTimeMs: 30,
    requestId: "third",
    executionId: "3",
    caller: "test",
    environment: "isolate",
    identityType: "user",
    parentExecutionId: null,
  },
  {
    id: "8",
    kind: "log",
    localizedTimestamp: "2022-06-01T23:18:59.467Z",
    timestamp: Date.parse("2022-06-01T23:18:59.467Z"),
    udfType: "Mutation",
    call: functionIdentifierValue("mutateData"),
    output: { level: "FAILURE", messages: ["Log!"], isTruncated: false },
    requestId: "fOuRtH",
    executionId: "4",
  },
  {
    id: "9",
    kind: "outcome",
    localizedTimestamp: "2022-06-01T23:18:59.467Z",
    timestamp: Date.parse("2022-06-01T23:18:59.467Z"),
    outcome: { status: "failure", statusCode: null },
    udfType: "Mutation",
    call: functionIdentifierValue("mutateData"),
    executionTimeMs: 30,
    requestId: "fOuRtH",
    executionId: "4",
    caller: "test",
    environment: "isolate",
    identityType: "user",
    parentExecutionId: null,
  },
  {
    id: "10",
    kind: "outcome",
    localizedTimestamp: "2022-06-01T23:18:59.467Z",
    timestamp: Date.parse("2022-06-01T23:18:59.467Z"),
    outcome: { status: "success", statusCode: null },
    udfType: "Mutation",
    call: functionIdentifierValue("mutateData"),
    executionTimeMs: 30,
    requestId: "fifth",
    executionId: "5",
    caller: "test",
    environment: "isolate",
    identityType: "user",
    parentExecutionId: null,
  },
];

const functions = ["queryData", "subquery", "mutateData"].map((identifier) =>
  functionIdentifierValue(identifier),
);
const statuses = ["success", "failure"];

describe("filterLogs", () => {
  it("should not filter when there is no filter", () => {
    expect(
      filterLogs(
        {
          logTypes: [...ALL_LEVELS, ...statuses],
          functions,
          selectedFunctions: functions,
          selectedNents: "all",
          filter: "",
        },
        logs,
      ),
    ).toEqual(logs);
  });

  it("should filter by log level", () => {
    expect(
      filterLogs(
        {
          logTypes: ALL_LEVELS.slice(1),
          functions,
          selectedFunctions: functions,
          selectedNents: "all",
          filter: "",
        },
        logs,
      ),
    ).toEqual([logs[2], logs[3], logs[5], logs[7]]);
  });

  it("should filter by status", () => {
    expect(
      filterLogs(
        {
          logTypes: ["failure"],
          functions,
          selectedFunctions: functions,
          selectedNents: "all",
          filter: "",
        },
        logs,
      ),
    ).toEqual([logs[8]]);
  });

  it("should filter by string for function name", () => {
    expect(
      filterLogs(
        {
          logTypes: [...ALL_LEVELS, ...statuses],
          functions,
          selectedFunctions: functions,
          selectedNents: "all",
          filter: "mutate",
        },
        logs,
      ),
    ).toEqual([logs[0], logs[1], logs[7], logs[8], logs[9]]);
  });

  it("should filter by string for text", () => {
    expect(
      filterLogs(
        {
          logTypes: [...ALL_LEVELS, ...statuses],
          functions,
          selectedFunctions: functions,
          selectedNents: "all",
          filter: "result",
        },
        logs,
      ),
    ).toEqual([logs[2]]);
  });

  it("should filter by level and string", () => {
    expect(
      filterLogs(
        {
          logTypes: ALL_LEVELS.slice(3),
          functions,
          selectedFunctions: functions,
          selectedNents: "all",
          filter: "query",
        },
        logs,
      ),
    ).toEqual([logs[5]]);
  });

  it("should filter by function", () => {
    expect(
      filterLogs(
        {
          logTypes: [...ALL_LEVELS, ...statuses],
          functions,
          selectedFunctions: [functionIdentifierValue("queryData")],
          selectedNents: "all",
          filter: "",
        },
        logs,
      ),
    ).toEqual([logs[2], logs[4], logs[5], logs[6]]);
  });

  it("should filter by subfunction name", () => {
    expect(
      filterLogs(
        {
          logTypes: [...ALL_LEVELS, ...statuses],
          functions,
          selectedFunctions: [functionIdentifierValue("subquery")],
          selectedNents: "all",
          filter: "",
        },
        logs,
      ),
    ).toEqual([logs[3]]);
  });

  it("should handle 'all' state for selectedFunctions", () => {
    expect(
      filterLogs(
        {
          logTypes: [...ALL_LEVELS, ...statuses],
          functions,
          selectedFunctions: "all",
          selectedNents: "all",
          filter: "",
        },
        logs,
      ),
    ).toEqual(logs);
  });

  it("should handle 'all' state for logTypes", () => {
    expect(
      filterLogs(
        {
          logTypes: "all",
          functions,
          selectedFunctions: functions,
          selectedNents: "all",
          filter: "",
        },
        logs,
      ),
    ).toEqual(logs);
  });

  it("should handle 'all' state for both logTypes and selectedFunctions", () => {
    expect(
      filterLogs(
        {
          logTypes: "all",
          functions,
          selectedFunctions: "all",
          selectedNents: "all",
          filter: "",
        },
        logs,
      ),
    ).toEqual(logs);
  });

  it("should not include rows from unknown functions if 'others' is not selected ", () => {
    expect(
      filterLogs(
        {
          logTypes: [...ALL_LEVELS, ...statuses],
          functions,
          selectedFunctions: [functionIdentifierValue("queryData")],
          selectedNents: "all",
          filter: "",
        },
        logs,
      ),
    ).toEqual([logs[2], logs[4], logs[5], logs[6]]);
  });

  it("should include rows from unknown functions if 'others' is selected ", () => {
    expect(
      filterLogs(
        {
          logTypes: [...ALL_LEVELS, ...statuses],
          functions: [functionIdentifierValue("queryData")],
          selectedFunctions: [
            functionIdentifierValue("queryData"),
            functionIdentifierValue("_other"),
          ],
          selectedNents: "all",
          filter: "",
        },
        logs,
      ),
    ).toEqual(logs);
  });

  it("should include rows matching the request id exactly", () => {
    expect(
      filterLogs(
        {
          logTypes: [...ALL_LEVELS, ...statuses],
          functions,
          selectedFunctions: [
            functionIdentifierValue("mutateData"),
            functionIdentifierValue("_other"),
          ],
          selectedNents: "all",
          filter: "fifth",
        },
        logs,
      ),
    ).toEqual([{ ...omit(logs[9], "output"), kind: "outcome" }]);
  });

  it("should include rows when the query contains the request id", () => {
    expect(
      filterLogs(
        {
          logTypes: [...ALL_LEVELS, ...statuses],
          functions,
          selectedFunctions: [
            functionIdentifierValue("mutateData"),
            functionIdentifierValue("_other"),
          ],
          selectedNents: "all",
          filter: "Request Id: fifth",
        },
        logs,
      ),
    ).toEqual([logs[9]]);
  });

  it("should match request ids case sensitively", () => {
    expect(
      filterLogs(
        {
          logTypes: [...ALL_LEVELS, ...statuses],
          functions,
          selectedFunctions: [
            functionIdentifierValue("mutateData"),
            functionIdentifierValue("_other"),
          ],
          selectedNents: "all",
          filter: "fOuRtH",
        },
        logs,
      ),
    ).toEqual([logs[7], logs[8]]);
    expect(
      filterLogs(
        {
          logTypes: [...ALL_LEVELS, ...statuses],
          functions,
          selectedFunctions: [
            functionIdentifierValue("queryData"),
            functionIdentifierValue("_other"),
          ],
          selectedNents: "all",
          filter: "fourth",
        },
        logs,
      ),
    ).toEqual([]);
  });
});

describe("filterLogs benchmark", () => {
  test("filterLogs with a large number of logs", () => {
    const largeLogs: UdfLog[] = [];
    for (let i = 0; i < 10000; i++) {
      largeLogs.push({
        id: `${i}`,
        kind: "log",
        timestamp: i,
        localizedTimestamp: new Date().toISOString(),
        udfType: "Mutation",
        call: functionIdentifierValue("mutateData"),
        output: { level: "DEBUG", messages: ["Log!"], isTruncated: false },
        requestId: `request-${i}`,
        executionId: `${i}`,
      });
    }

    const start = performance.now();
    filterLogs(
      {
        logTypes: [...ALL_LEVELS, ...statuses],
        selectedFunctions: functions,
        functions,
        selectedNents: "all",
        filter: "",
      },
      largeLogs,
    );
    const end = performance.now();

    // eslint-disable-next-line no-console
    console.log(`Benchmark took ${end - start} milliseconds`);
  });
});

describe("filterLogs nents filtering", () => {
  // Helper to create function identifier with nent
  function nentFunction(name: string, nent: string) {
    return functionIdentifierValue(name, nent);
  }

  const nentA = "nentA";
  const nentB = "nentB";
  const nentC = "nentC";
  const nentLogs: UdfLog[] = [
    {
      id: "a1",
      kind: "log",
      timestamp: 1,
      localizedTimestamp: "2022-06-01T23:18:59.467Z",
      udfType: "Mutation",
      call: nentFunction("mutateData", nentA),
      output: { level: "DEBUG", messages: ["A log!"], isTruncated: false },
      requestId: "a1",
      executionId: "a1",
    },
    {
      id: "b1",
      kind: "log",
      timestamp: 2,
      localizedTimestamp: "2022-06-01T23:18:59.467Z",
      udfType: "Mutation",
      call: nentFunction("mutateData", nentB),
      output: { level: "DEBUG", messages: ["B log!"], isTruncated: false },
      requestId: "b1",
      executionId: "b1",
    },
    {
      id: "c1",
      kind: "log",
      timestamp: 3,
      localizedTimestamp: "2022-06-01T23:18:59.467Z",
      udfType: "Mutation",
      call: nentFunction("mutateData", nentC),
      output: { level: "DEBUG", messages: ["C log!"], isTruncated: false },
      requestId: "c1",
      executionId: "c1",
    },
  ];
  const nentFunctions = [
    nentFunction("mutateData", nentA),
    nentFunction("mutateData", nentB),
    nentFunction("mutateData", nentC),
  ];

  it("should filter logs to only those in selectedNents (single nent)", () => {
    expect(
      filterLogs(
        {
          logTypes: [...ALL_LEVELS],
          functions: nentFunctions,
          selectedFunctions: nentFunctions,
          selectedNents: [nentA],
          filter: "",
        },
        nentLogs,
      ),
    ).toEqual([nentLogs[0]]);
  });

  it("should filter logs to only those in selectedNents (multiple nents)", () => {
    expect(
      filterLogs(
        {
          logTypes: [...ALL_LEVELS],
          functions: nentFunctions,
          selectedFunctions: nentFunctions,
          selectedNents: [nentA, nentB],
          filter: "",
        },
        nentLogs,
      ),
    ).toEqual([nentLogs[0], nentLogs[1]]);
  });

  it("should return no logs if selectedNents does not match any log's nent", () => {
    expect(
      filterLogs(
        {
          logTypes: [...ALL_LEVELS],
          functions: nentFunctions,
          selectedFunctions: nentFunctions,
          selectedNents: ["nonexistentNent"],
          filter: "",
        },
        nentLogs,
      ),
    ).toEqual([]);
  });

  it("should return all logs if selectedNents is 'all'", () => {
    expect(
      filterLogs(
        {
          logTypes: [...ALL_LEVELS],
          functions: nentFunctions,
          selectedFunctions: nentFunctions,
          selectedNents: "all",
          filter: "",
        },
        nentLogs,
      ),
    ).toEqual(nentLogs);
  });

  it("should filter logs by nent when selectedFunctions is 'all'", () => {
    expect(
      filterLogs(
        {
          logTypes: [...ALL_LEVELS],
          functions: nentFunctions,
          selectedFunctions: "all",
          selectedNents: [nentB],
          filter: "",
        },
        nentLogs,
      ),
    ).toEqual([nentLogs[1]]);
  });

  it("should include logs with NENT_APP_PLACEHOLDER if selectedNents includes it", () => {
    const appLog: UdfLog = {
      id: "app1",
      kind: "log",
      timestamp: 4,
      localizedTimestamp: "2022-06-01T23:18:59.467Z",
      udfType: "Mutation",
      call: functionIdentifierValue("mutateData", ""),
      output: { level: "DEBUG", messages: ["App log!"], isTruncated: false },
      requestId: "app1",
      executionId: "app1",
    };
    expect(
      filterLogs(
        {
          logTypes: [...ALL_LEVELS],
          functions: [appLog.call],
          selectedFunctions: [appLog.call],
          selectedNents: [NENT_APP_PLACEHOLDER],
          filter: "",
        },
        [appLog],
      ),
    ).toEqual([appLog]);
    expect(
      filterLogs(
        {
          logTypes: [...ALL_LEVELS],
          functions: [appLog.call],
          selectedFunctions: [appLog.call],
          selectedNents: ["not_app"],
          filter: "",
        },
        [appLog],
      ),
    ).toEqual([]);
  });

  it("should filter by subfunction nent if present", () => {
    const subLog: UdfLog = {
      id: "sub1",
      kind: "log",
      timestamp: 5,
      localizedTimestamp: "2022-06-01T23:18:59.467Z",
      udfType: "Mutation",
      call: nentFunction("mutateData", nentA),
      output: {
        level: "DEBUG",
        messages: ["Sub log!"],
        isTruncated: false,
        subfunction: functionIdentifierValue("subFunc", nentB ?? ""),
      },
      requestId: "sub1",
      executionId: "sub1",
    };
    expect(
      filterLogs(
        {
          logTypes: [...ALL_LEVELS],
          functions: [subLog.call, subLog.output.subfunction as string],
          selectedFunctions: [subLog.call, subLog.output.subfunction as string],
          selectedNents: [nentB],
          filter: "",
        },
        [subLog],
      ),
    ).toEqual([subLog]);
    expect(
      filterLogs(
        {
          logTypes: [...ALL_LEVELS],
          functions: [subLog.call, subLog.output.subfunction as string],
          selectedFunctions: [subLog.call, subLog.output.subfunction as string],
          selectedNents: [nentA],
          filter: "",
        },
        [subLog],
      ),
    ).toEqual([]);
  });

  it("should return no logs if text filter matches only logs outside selectedNents", () => {
    expect(
      filterLogs(
        {
          logTypes: [...ALL_LEVELS],
          functions: nentFunctions,
          selectedFunctions: nentFunctions,
          selectedNents: [nentA],
          filter: "B log!",
        },
        nentLogs,
      ),
    ).toEqual([]);
  });

  it("should filter by both nent and log level", () => {
    const errorLog: UdfLog = {
      id: "err1",
      kind: "log",
      timestamp: 6,
      localizedTimestamp: "2022-06-01T23:18:59.467Z",
      udfType: "Mutation",
      call: nentFunction("mutateData", nentA),
      output: { level: "ERROR", messages: ["Error!"], isTruncated: false },
      requestId: "err1",
      executionId: "err1",
    };
    expect(
      filterLogs(
        {
          logTypes: ["ERROR"],
          functions: [errorLog.call],
          selectedFunctions: [errorLog.call],
          selectedNents: [nentA],
          filter: "",
        },
        [errorLog],
      ),
    ).toEqual([errorLog]);
    expect(
      filterLogs(
        {
          logTypes: ["ERROR"],
          functions: [errorLog.call],
          selectedFunctions: [errorLog.call],
          selectedNents: [nentB],
          filter: "",
        },
        [errorLog],
      ),
    ).toEqual([]);
  });

  it("should return no logs if selectedNents is empty array", () => {
    expect(
      filterLogs(
        {
          logTypes: [...ALL_LEVELS],
          functions: nentFunctions,
          selectedFunctions: nentFunctions,
          selectedNents: [],
          filter: "",
        },
        nentLogs,
      ),
    ).toEqual([]);
  });
});
