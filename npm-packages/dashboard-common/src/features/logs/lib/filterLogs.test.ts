import omit from "lodash/omit";
import { filterLogs, ALL_LEVELS } from "@common/features/logs/lib/filterLogs";
import { functionIdentifierValue } from "@common/lib/functions/generateFileTree";
import { UdfLog } from "@common/lib/useLogs";

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
          filter: "",
        },
        logs,
      ),
    ).toEqual([logs[3]]);
  });

  it("should not include rows from unknown functions if “others” is not selected ", () => {
    expect(
      filterLogs(
        {
          logTypes: [...ALL_LEVELS, ...statuses],
          functions: [functionIdentifierValue("queryData")],
          selectedFunctions: [functionIdentifierValue("queryData")],
          filter: "",
        },
        logs,
      ),
    ).toEqual([logs[2], logs[4], logs[5], logs[6]]);
  });

  it("should include rows from unknown functions if “others” is selected ", () => {
    expect(
      filterLogs(
        {
          logTypes: [...ALL_LEVELS, ...statuses],
          functions: [functionIdentifierValue("queryData")],
          selectedFunctions: [
            functionIdentifierValue("queryData"),
            functionIdentifierValue("_other"),
          ],
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
          functions: [functionIdentifierValue("queryData")],
          selectedFunctions: [
            functionIdentifierValue("queryData"),
            functionIdentifierValue("_other"),
          ],
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
          functions: [functionIdentifierValue("queryData")],
          selectedFunctions: [
            functionIdentifierValue("queryData"),
            functionIdentifierValue("_other"),
          ],
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
          functions: [functionIdentifierValue("queryData")],
          selectedFunctions: [
            functionIdentifierValue("queryData"),
            functionIdentifierValue("_other"),
          ],
          filter: "fOuRtH",
        },
        logs,
      ),
    ).toEqual([logs[7], logs[8]]);
    expect(
      filterLogs(
        {
          logTypes: [...ALL_LEVELS, ...statuses],
          functions: [functionIdentifierValue("queryData")],
          selectedFunctions: [
            functionIdentifierValue("queryData"),
            functionIdentifierValue("_other"),
          ],
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
        filter: "",
      },
      largeLogs,
    );
    const end = performance.now();

    // eslint-disable-next-line no-console
    console.log(`Benchmark took ${end - start} milliseconds`);
  });
});
