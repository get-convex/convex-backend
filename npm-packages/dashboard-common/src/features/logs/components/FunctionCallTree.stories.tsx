import type { Meta, StoryObj } from "@storybook/nextjs";
import { functionIdentifierValue } from "@common/lib/functions/generateFileTree";
import { FunctionCallTree } from "./FunctionCallTree";

const meta: Meta<typeof FunctionCallTree> = {
  component: FunctionCallTree,
  parameters: {
    layout: "padded",
  },
};

export default meta;
type Story = StoryObj<typeof meta>;

const mockLogs = [
  {
    id: "1",
    udfType: "Query" as const,
    localizedTimestamp: "2023-10-15 10:30:00",
    timestamp: 1697365800000,
    call: functionIdentifierValue("api/users:list"),
    cachedResult: false,
    requestId: "req_123",
    executionId: "exec_1",
    caller: "dashboard",
    environment: "production",
    identityType: "user",
    parentExecutionId: null,
    executionTimestamp: 1697365800000,
    kind: "outcome" as const,
    outcome: {
      status: "success" as const,
      statusCode: null,
    },
    executionTimeMs: 3000,
  },
  {
    id: "2",
    udfType: "Mutation" as const,
    localizedTimestamp: "2023-10-15 10:30:05",
    timestamp: 1697365805000,
    call: functionIdentifierValue("api/users:create"),
    cachedResult: false,
    requestId: "req_124",
    executionId: "exec_2",
    caller: "dashboard",
    environment: "production",
    identityType: "user",
    parentExecutionId: "exec_1",
    executionTimestamp: 1697365805000,
    kind: "outcome" as const,
    outcome: {
      status: "failure" as const,
      statusCode: null,
    },
    executionTimeMs: 2500,
    error: "User creation failed: Database constraint violation",
  },
  {
    id: "3",
    udfType: "Action" as const,
    localizedTimestamp: "2023-10-15 10:30:10",
    timestamp: 1697365810000,
    call: functionIdentifierValue("api/email:send"),
    cachedResult: false,
    requestId: "req_125",
    executionId: "exec_3",
    caller: "dashboard",
    environment: "production",
    identityType: "user",
    parentExecutionId: "exec_2",
    executionTimestamp: 1697365810000,
    kind: "outcome" as const,
    outcome: {
      status: "failure" as const,
      statusCode: null,
    },
    executionTimeMs: 800,
    error: "Failed to send email: Invalid recipient address",
  },
];

const mockRunningLogs = [
  {
    id: "4",
    udfType: "Query" as const,
    timestamp: Date.now(),
    call: functionIdentifierValue("api/users:search"),
    executionId: "exec_4",
    localizedTimestamp: new Date().toISOString(),
    requestId: "req_4",
    kind: "log" as const,
    output: {
      isTruncated: false,
      messages: ["Starting search..."],
      level: "INFO" as const,
    },
  },
];

export const WithSuccessfulFunctions: Story = {
  args: {
    logs: [
      // Log messages for exec_1
      {
        id: "log_1_1",
        udfType: "Query" as const,
        localizedTimestamp: "2023-10-15 10:30:00",
        timestamp: 1697365800000,
        call: functionIdentifierValue("api/users:list"),
        requestId: "req_123",
        executionId: "exec_1",
        kind: "log" as const,
        output: {
          level: "INFO" as const,
          messages: ["Starting user list query"],
          isTruncated: false,
        },
      },
      {
        id: "log_1_2",
        udfType: "Query" as const,
        localizedTimestamp: "2023-10-15 10:30:00",
        timestamp: 1697365800100,
        call: functionIdentifierValue("api/users:list"),
        requestId: "req_123",
        executionId: "exec_1",
        kind: "log" as const,
        output: {
          level: "DEBUG" as const,
          messages: ["Found 25 users"],
          isTruncated: false,
        },
      },
      {
        id: "1",
        udfType: "Query" as const,
        localizedTimestamp: "2023-10-15 10:30:00",
        timestamp: 1697365800000,
        call: functionIdentifierValue("api/users:list"),
        cachedResult: false,
        requestId: "req_123",
        executionId: "exec_1",
        caller: "dashboard",
        environment: "production",
        identityType: "user",
        parentExecutionId: null,
        executionTimestamp: 1697365800000,
        kind: "outcome" as const,
        outcome: {
          status: "success" as const,
          statusCode: null,
        },
        executionTimeMs: 45,
      },
      // Log messages for exec_2
      {
        id: "log_2_1",
        udfType: "Mutation" as const,
        localizedTimestamp: "2023-10-15 10:30:05",
        timestamp: 1697365805000,
        call: functionIdentifierValue("api/users:update"),
        requestId: "req_124",
        executionId: "exec_2",
        kind: "log" as const,
        output: {
          level: "INFO" as const,
          messages: ["Starting user update"],
          isTruncated: false,
        },
      },
      {
        id: "2",
        udfType: "Mutation" as const,
        localizedTimestamp: "2023-10-15 10:30:05",
        timestamp: 1697365805000,
        call: functionIdentifierValue("api/users:update"),
        cachedResult: false,
        requestId: "req_124",
        executionId: "exec_2",
        caller: "dashboard",
        environment: "production",
        identityType: "user",
        parentExecutionId: "exec_1",
        executionTimestamp: 1697365805000,
        kind: "outcome" as const,
        outcome: {
          status: "success" as const,
          statusCode: null,
        },
        executionTimeMs: 120,
      },
    ],
    onFunctionSelect: (executionId: string, functionName: string) => {
      alert(`Selected function: ${functionName} (${executionId})`);
    },
  },
};

export const WithFailedFunction: Story = {
  args: {
    logs: [
      {
        id: "1",
        udfType: "Query" as const,
        localizedTimestamp: "2023-10-15 10:30:00",
        timestamp: 1697365800000,
        call: functionIdentifierValue("api/users:list"),
        cachedResult: false,
        requestId: "req_123",
        executionId: "exec_1",
        caller: "dashboard",
        environment: "production",
        identityType: "user",
        parentExecutionId: null,
        executionTimestamp: 1697365800000,
        kind: "outcome" as const,
        outcome: {
          status: "failure" as const,
          statusCode: null,
        },
        executionTimeMs: 1500,
        error: "User validation failed",
      },
      {
        id: "2",
        udfType: "Action" as const,
        localizedTimestamp: "2023-10-15 10:30:05",
        timestamp: 1697365805000,
        call: functionIdentifierValue("api/payment:process"),
        cachedResult: false,
        requestId: "req_124",
        executionId: "exec_2",
        caller: "dashboard",
        environment: "production",
        identityType: "user",
        parentExecutionId: "exec_1",
        executionTimestamp: 1697365805000,
        kind: "outcome" as const,
        outcome: {
          status: "failure" as const,
          statusCode: null,
        },
        executionTimeMs: 800,
        error: "Payment gateway timeout",
      },
    ],
    onFunctionSelect: (executionId: string, functionName: string) => {
      alert(`Selected function: ${functionName} (${executionId})`);
    },
  },
};

export const WithRunningFunction: Story = {
  args: {
    logs: [
      {
        id: "1",
        udfType: "Action" as const,
        timestamp: Date.now() - 5000,
        call: functionIdentifierValue("api/batch:process"),
        executionId: "exec_1",
        localizedTimestamp: "2023-10-15 10:30:00",
        requestId: "req_123",
        kind: "log" as const,
        output: {
          isTruncated: false,
          messages: ["Starting batch processing..."],
          level: "INFO" as const,
        },
      },
      {
        id: "1_2",
        udfType: "Action" as const,
        timestamp: Date.now() - 4000,
        call: functionIdentifierValue("api/batch:process"),
        executionId: "exec_1",
        localizedTimestamp: "2023-10-15 10:30:01",
        requestId: "req_123",
        kind: "log" as const,
        output: {
          isTruncated: false,
          messages: ["Processing 100 items..."],
          level: "DEBUG" as const,
        },
      },
      {
        id: "1_3",
        udfType: "Action" as const,
        timestamp: Date.now() - 3000,
        call: functionIdentifierValue("api/batch:process"),
        executionId: "exec_1",
        localizedTimestamp: "2023-10-15 10:30:02",
        requestId: "req_123",
        kind: "log" as const,
        output: {
          isTruncated: false,
          messages: ["Processed 50 items so far..."],
          level: "INFO" as const,
        },
      },
      {
        id: "2",
        udfType: "Query" as const,
        localizedTimestamp: "2023-10-15 10:30:01",
        timestamp: 1697365801000,
        call: functionIdentifierValue("api/data:validate"),
        cachedResult: false,
        requestId: "req_124",
        executionId: "exec_2",
        caller: "dashboard",
        environment: "production",
        identityType: "user",
        parentExecutionId: "exec_1",
        executionTimestamp: 1697365801000,
        kind: "outcome" as const,
        outcome: {
          status: "success" as const,
          statusCode: null,
        },
        executionTimeMs: 150,
      },
      {
        id: "3",
        udfType: "Mutation" as const,
        timestamp: Date.now() - 2000,
        call: functionIdentifierValue("api/data:update"),
        executionId: "exec_3",
        localizedTimestamp: "2023-10-15 10:30:03",
        requestId: "req_125",
        kind: "log" as const,
        output: {
          isTruncated: false,
          messages: ["Updating batch data..."],
          level: "INFO" as const,
        },
      },
    ],
    onFunctionSelect: (executionId: string, functionName: string) => {
      alert(`Selected function: ${functionName} (${executionId})`);
    },
  },
};

export const WithNestedFunctions: Story = {
  args: {
    logs: mockLogs,
    onFunctionSelect: (executionId: string, functionName: string) => {
      alert(`Selected function: ${functionName} (${executionId})`);
    },
  },
};

export const WithComplexNesting: Story = {
  args: {
    logs: [
      {
        id: "1",
        udfType: "Query" as const,
        localizedTimestamp: "2023-10-15 10:30:00",
        timestamp: 1697365800000,
        call: functionIdentifierValue("api/orders:process"),
        cachedResult: false,
        requestId: "req_123",
        executionId: "exec_1",
        caller: "dashboard",
        environment: "production",
        identityType: "user",
        parentExecutionId: null,
        executionTimestamp: 1697365800000,
        kind: "outcome" as const,
        outcome: {
          status: "success" as const,
          statusCode: null,
        },
        executionTimeMs: 2500,
      },
      {
        id: "2",
        udfType: "Query" as const,
        localizedTimestamp: "2023-10-15 10:30:01",
        timestamp: 1697365801000,
        call: functionIdentifierValue("api/inventory:check"),
        cachedResult: false,
        requestId: "req_124",
        executionId: "exec_2",
        caller: "dashboard",
        environment: "production",
        identityType: "user",
        parentExecutionId: "exec_1",
        executionTimestamp: 1697365801000,
        kind: "outcome" as const,
        outcome: {
          status: "success" as const,
          statusCode: null,
        },
        executionTimeMs: 150,
      },
      {
        id: "3",
        udfType: "Mutation" as const,
        localizedTimestamp: "2023-10-15 10:30:02",
        timestamp: 1697365802000,
        call: functionIdentifierValue("api/inventory:reserve"),
        cachedResult: false,
        requestId: "req_125",
        executionId: "exec_3",
        caller: "dashboard",
        environment: "production",
        identityType: "user",
        parentExecutionId: "exec_2",
        executionTimestamp: 1697365802000,
        kind: "outcome" as const,
        outcome: {
          status: "success" as const,
          statusCode: null,
        },
        executionTimeMs: 300,
      },
      {
        id: "4",
        udfType: "Action" as const,
        localizedTimestamp: "2023-10-15 10:30:03",
        timestamp: 1697365803000,
        call: functionIdentifierValue("api/payment:charge"),
        cachedResult: false,
        requestId: "req_126",
        executionId: "exec_4",
        caller: "dashboard",
        environment: "production",
        identityType: "user",
        parentExecutionId: "exec_1",
        executionTimestamp: 1697365803000,
        kind: "outcome" as const,
        outcome: {
          status: "success" as const,
          statusCode: null,
        },
        executionTimeMs: 800,
      },
      {
        id: "5",
        udfType: "Action" as const,
        localizedTimestamp: "2023-10-15 10:30:04",
        timestamp: 1697365804000,
        call: functionIdentifierValue("api/email:confirmation"),
        cachedResult: false,
        requestId: "req_127",
        executionId: "exec_5",
        caller: "dashboard",
        environment: "production",
        identityType: "user",
        parentExecutionId: "exec_4",
        executionTimestamp: 1697365804000,
        kind: "outcome" as const,
        outcome: {
          status: "success" as const,
          statusCode: null,
        },
        executionTimeMs: 400,
      },
    ],
    onFunctionSelect: (executionId: string, functionName: string) => {
      alert(`Selected function: ${functionName} (${executionId})`);
    },
  },
};
