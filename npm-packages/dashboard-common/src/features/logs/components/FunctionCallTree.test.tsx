import { UdfLog } from "@common/lib/useLogs";
import { functionIdentifierValue } from "@common/lib/functions/generateFileTree";
import { createExecutionNodes } from "./FunctionCallTree";

describe("createExecutionNodes", () => {
  it("should handle empty logs array", () => {
    const result = createExecutionNodes([]);
    expect(result).toEqual([]);
  });

  it("should create a single completed execution node", () => {
    const logs: UdfLog[] = [
      {
        id: "log1",
        kind: "log",
        timestamp: 1000,
        localizedTimestamp: "2023-10-15 10:30:00",
        udfType: "Query",
        call: functionIdentifierValue("api/users:list"),
        requestId: "req_1",
        executionId: "exec_1",
        output: {
          level: "INFO",
          messages: ["Starting query"],
          isTruncated: false,
        },
      },
      {
        id: "outcome1",
        kind: "outcome",
        timestamp: 2000,
        localizedTimestamp: "2023-10-15 10:30:01",
        udfType: "Query",
        call: functionIdentifierValue("api/users:list"),
        requestId: "req_1",
        executionId: "exec_1",
        outcome: {
          status: "success",
          statusCode: null,
        },
        executionTimeMs: 1000,
        caller: "dashboard",
        environment: "production",
        identityType: "user",
        parentExecutionId: null,
        executionTimestamp: 1000,
      },
    ];

    const result = createExecutionNodes(logs);

    expect(result).toHaveLength(1);
    expect(result[0]).toMatchObject({
      executionId: "exec_1",
      functionName: functionIdentifierValue("api/users:list"),
      status: "success",
      executionTime: 1000,
      logCount: 1,
      children: [],
    });
  });

  it("should create a running execution node from logs without outcome", () => {
    const logs: UdfLog[] = [
      {
        id: "log1",
        kind: "log",
        timestamp: 1000,
        localizedTimestamp: "2023-10-15 10:30:00",
        udfType: "Action",
        call: functionIdentifierValue("api/batch:process"),
        requestId: "req_1",
        executionId: "exec_1",
        output: {
          level: "INFO",
          messages: ["Starting batch process"],
          isTruncated: false,
        },
      },
      {
        id: "log2",
        kind: "log",
        timestamp: 2000,
        localizedTimestamp: "2023-10-15 10:30:01",
        udfType: "Action",
        call: functionIdentifierValue("api/batch:process"),
        requestId: "req_1",
        executionId: "exec_1",
        output: {
          level: "DEBUG",
          messages: ["Processing item 1"],
          isTruncated: false,
        },
      },
    ];

    const result = createExecutionNodes(logs);

    expect(result).toHaveLength(1);
    expect(result[0]).toMatchObject({
      executionId: "exec_1",
      functionName: functionIdentifierValue("api/batch:process"),
      status: "running",
      executionTime: undefined,
      logCount: 2,
      children: [],
    });
  });

  it("should correctly count logs per execution", () => {
    const logs: UdfLog[] = [
      // Execution 1: 3 logs
      {
        id: "log1_1",
        kind: "log",
        timestamp: 1000,
        localizedTimestamp: "2023-10-15 10:30:00",
        udfType: "Query",
        call: functionIdentifierValue("api/users:list"),
        requestId: "req_1",
        executionId: "exec_1",
        output: { level: "INFO", messages: ["Log 1"], isTruncated: false },
      },
      {
        id: "log1_2",
        kind: "log",
        timestamp: 1100,
        localizedTimestamp: "2023-10-15 10:30:01",
        udfType: "Query",
        call: functionIdentifierValue("api/users:list"),
        requestId: "req_1",
        executionId: "exec_1",
        output: { level: "DEBUG", messages: ["Log 2"], isTruncated: false },
      },
      {
        id: "log1_3",
        kind: "log",
        timestamp: 1200,
        localizedTimestamp: "2023-10-15 10:30:02",
        udfType: "Query",
        call: functionIdentifierValue("api/users:list"),
        requestId: "req_1",
        executionId: "exec_1",
        output: { level: "INFO", messages: ["Log 3"], isTruncated: false },
      },
      {
        id: "outcome1",
        kind: "outcome",
        timestamp: 2000,
        localizedTimestamp: "2023-10-15 10:30:03",
        udfType: "Query",
        call: functionIdentifierValue("api/users:list"),
        requestId: "req_1",
        executionId: "exec_1",
        outcome: { status: "success", statusCode: null },
        executionTimeMs: 1000,
        caller: "dashboard",
        environment: "production",
        identityType: "user",
        parentExecutionId: null,
        executionTimestamp: 1000,
      },
      // Execution 2: 1 log
      {
        id: "log2_1",
        kind: "log",
        timestamp: 3000,
        localizedTimestamp: "2023-10-15 10:30:04",
        udfType: "Mutation",
        call: functionIdentifierValue("api/users:update"),
        requestId: "req_2",
        executionId: "exec_2",
        output: {
          level: "INFO",
          messages: ["Updating user"],
          isTruncated: false,
        },
      },
      {
        id: "outcome2",
        kind: "outcome",
        timestamp: 4000,
        localizedTimestamp: "2023-10-15 10:30:05",
        udfType: "Mutation",
        call: functionIdentifierValue("api/users:update"),
        requestId: "req_2",
        executionId: "exec_2",
        outcome: { status: "failure", statusCode: null },
        executionTimeMs: 500,
        caller: "dashboard",
        environment: "production",
        identityType: "user",
        parentExecutionId: null,
        executionTimestamp: 3000,
        error: "User not found",
      },
    ];

    const result = createExecutionNodes(logs);

    expect(result).toHaveLength(2);

    const exec1 = result.find((n) => n.executionId === "exec_1");
    const exec2 = result.find((n) => n.executionId === "exec_2");

    expect(exec1?.logCount).toBe(3);
    expect(exec2?.logCount).toBe(1);
  });

  it("should build parent-child relationships correctly", () => {
    const logs: UdfLog[] = [
      // Parent execution
      {
        id: "outcome1",
        kind: "outcome",
        timestamp: 3000,
        localizedTimestamp: "2023-10-15 10:30:03",
        udfType: "Query",
        call: functionIdentifierValue("api/parent"),
        requestId: "req_1",
        executionId: "exec_1",
        outcome: { status: "success", statusCode: null },
        executionTimeMs: 2000,
        caller: "dashboard",
        environment: "production",
        identityType: "user",
        parentExecutionId: null,
        executionTimestamp: 1000,
      },
      // Child execution
      {
        id: "outcome2",
        kind: "outcome",
        timestamp: 2500,
        localizedTimestamp: "2023-10-15 10:30:02",
        udfType: "Mutation",
        call: functionIdentifierValue("api/child"),
        requestId: "req_2",
        executionId: "exec_2",
        outcome: { status: "success", statusCode: null },
        executionTimeMs: 500,
        caller: "dashboard",
        environment: "production",
        identityType: "user",
        parentExecutionId: "exec_1",
        executionTimestamp: 2000,
      },
    ];

    const result = createExecutionNodes(logs);

    expect(result).toHaveLength(1); // Only parent should be at root level
    expect(result[0].executionId).toBe("exec_1");
    expect(result[0].children).toHaveLength(1);
    expect(result[0].children[0].executionId).toBe("exec_2");
    expect(result[0].children[0].parentExecutionId).toBe("exec_1");
  });

  it("should sort executions by start time", () => {
    const logs: UdfLog[] = [
      // Second execution (starts later)
      {
        id: "outcome2",
        kind: "outcome",
        timestamp: 3000,
        localizedTimestamp: "2023-10-15 10:30:03",
        udfType: "Query",
        call: functionIdentifierValue("api/second"),
        requestId: "req_2",
        executionId: "exec_2",
        outcome: { status: "success", statusCode: null },
        executionTimeMs: 1000,
        caller: "dashboard",
        environment: "production",
        identityType: "user",
        parentExecutionId: null,
        executionTimestamp: 2000, // Starts at 2000
      },
      // First execution (starts earlier)
      {
        id: "outcome1",
        kind: "outcome",
        timestamp: 2000,
        localizedTimestamp: "2023-10-15 10:30:02",
        udfType: "Mutation",
        call: functionIdentifierValue("api/first"),
        requestId: "req_1",
        executionId: "exec_1",
        outcome: { status: "success", statusCode: null },
        executionTimeMs: 500,
        caller: "dashboard",
        environment: "production",
        identityType: "user",
        parentExecutionId: null,
        executionTimestamp: 1000, // Starts at 1000
      },
    ];

    const result = createExecutionNodes(logs);

    expect(result).toHaveLength(2);
    expect(result[0].executionId).toBe("exec_1"); // Should be first (starts at 1000)
    expect(result[1].executionId).toBe("exec_2"); // Should be second (starts at 2000)
  });
});
