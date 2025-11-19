import { renderHook, act } from "@testing-library/react";
import { createRef } from "react";
import { UdfLog } from "@common/lib/useLogs";
import { functionIdentifierValue } from "@common/lib/functions/generateFileTree";
import { InterleavedLog } from "../lib/interleaveLogs";

// Mock react-hotkeys-hook
const mockHotkeys: Record<string, () => void> = {};
jest.mock("react-hotkeys-hook", () => ({
  useHotkeys: jest.fn((keys: string | string[], callback: () => void) => {
    const keyString = Array.isArray(keys) ? keys[0] : keys;
    mockHotkeys[keyString] = callback;
  }),
}));

// Import after mocking
// eslint-disable-next-line import/first
import { useNavigateLogs } from "./LogDrilldown";

// Helper to convert UdfLog to InterleavedLog
function toInterleavedLog(log: UdfLog): InterleavedLog {
  return {
    kind: "ExecutionLog",
    executionLog: log,
  };
}

describe("useNavigateLogs", () => {
  const createLog = (
    timestamp: number,
    requestId: string,
    executionId: string,
    kind: "log" | "outcome" = "log",
  ): UdfLog => {
    const base = {
      id: `log_${timestamp}`,
      timestamp,
      localizedTimestamp: new Date(timestamp).toISOString(),
      udfType: "Query" as const,
      call: functionIdentifierValue("api/test"),
      requestId,
      executionId,
    };

    if (kind === "log") {
      return {
        ...base,
        kind: "log",
        output: {
          level: "INFO",
          messages: [`Log ${timestamp}`],
          isTruncated: false,
        },
      };
    }
    return {
      ...base,
      kind: "outcome",
      outcome: { status: "success", statusCode: null },
      executionTimeMs: 100,
      caller: "dashboard",
      environment: "isolate",
      identityType: "user",
      parentExecutionId: null,
      executionTimestamp: timestamp - 100,
    };
  };

  beforeEach(() => {
    // Clear all mocks before each test
    Object.keys(mockHotkeys).forEach((key) => delete mockHotkeys[key]);
  });

  describe("navigation within all logs", () => {
    it("should navigate to the next log when down arrow is pressed", () => {
      const logs: UdfLog[] = [
        createLog(3000, "req1", "exec1"),
        createLog(2000, "req1", "exec1"),
        createLog(1000, "req1", "exec1"),
      ];
      const interleavedLogs = logs.map(toInterleavedLog);
      const onSelectLog = jest.fn();
      const onHitBoundary = jest.fn();
      const selectedLog = interleavedLogs[1]; // timestamp 2000
      const rightPanelRef = createRef<HTMLDivElement>();

      renderHook(() =>
        useNavigateLogs(
          selectedLog,
          interleavedLogs,
          onSelectLog,
          jest.fn(),
          onHitBoundary,
          rightPanelRef,
        ),
      );

      // Trigger the down arrow hotkey
      act(() => {
        mockHotkeys.down?.();
      });

      // Should select the next log (timestamp 1000) and clear boundary
      expect(onSelectLog).toHaveBeenCalledWith(interleavedLogs[2]);
      expect(onHitBoundary).toHaveBeenCalledWith(null);
    });

    it("should navigate to the previous log when up arrow is pressed", () => {
      const logs: UdfLog[] = [
        createLog(3000, "req1", "exec1"),
        createLog(2000, "req1", "exec1"),
        createLog(1000, "req1", "exec1"),
      ];
      const onSelectLog = jest.fn();
      const onHitBoundary = jest.fn();
      const interleavedLogs = logs.map(toInterleavedLog);
      const selectedLog = interleavedLogs[1]; // timestamp 2000

      const rightPanelRef = createRef<HTMLDivElement>();

      renderHook(() =>
        useNavigateLogs(
          selectedLog,
          interleavedLogs,
          onSelectLog,
          jest.fn(),
          onHitBoundary,
          rightPanelRef,
        ),
      );

      // Trigger the up arrow hotkey
      act(() => {
        mockHotkeys.up?.();
      });

      // Should select the previous log (timestamp 3000) and clear boundary
      expect(onSelectLog).toHaveBeenCalledWith(interleavedLogs[0]);
      expect(onHitBoundary).toHaveBeenCalledWith(null);
    });

    it("should trigger bottom boundary when at the last log", () => {
      const logs: UdfLog[] = [
        createLog(3000, "req1", "exec1"),
        createLog(2000, "req1", "exec1"),
        createLog(1000, "req1", "exec1"),
      ];
      const onSelectLog = jest.fn();
      const onHitBoundary = jest.fn();
      const interleavedLogs = logs.map(toInterleavedLog);
      const selectedLog = interleavedLogs[2]; // timestamp 1000 (last log)

      const rightPanelRef = createRef<HTMLDivElement>();

      renderHook(() =>
        useNavigateLogs(
          selectedLog,
          interleavedLogs,
          onSelectLog,
          jest.fn(),
          onHitBoundary,
          rightPanelRef,
        ),
      );

      // Trigger the down arrow hotkey
      act(() => {
        mockHotkeys.down?.();
      });

      // Should hit the bottom boundary
      expect(onSelectLog).not.toHaveBeenCalled();
      expect(onHitBoundary).toHaveBeenCalledWith("bottom");
    });

    it("should trigger top boundary when at the first log", () => {
      const logs: UdfLog[] = [
        createLog(3000, "req1", "exec1"),
        createLog(2000, "req1", "exec1"),
        createLog(1000, "req1", "exec1"),
      ];
      const onSelectLog = jest.fn();
      const onHitBoundary = jest.fn();
      const interleavedLogs = logs.map(toInterleavedLog);
      const selectedLog = interleavedLogs[0]; // timestamp 3000 (first log)

      const rightPanelRef = createRef<HTMLDivElement>();

      renderHook(() =>
        useNavigateLogs(
          selectedLog,
          interleavedLogs,
          onSelectLog,
          jest.fn(),
          onHitBoundary,
          rightPanelRef,
        ),
      );

      // Trigger the up arrow hotkey
      act(() => {
        mockHotkeys.up?.();
      });

      // Should hit the top boundary
      expect(onSelectLog).not.toHaveBeenCalled();
      expect(onHitBoundary).toHaveBeenCalledWith("top");
    });
  });

  describe("navigation within request scope", () => {
    it("should navigate only within the same request with shift+down", () => {
      const logs: UdfLog[] = [
        createLog(4000, "req2", "exec2"),
        createLog(3000, "req1", "exec1"),
        createLog(2000, "req1", "exec1"),
        createLog(1000, "req2", "exec2"),
      ];
      const onSelectLog = jest.fn();
      const onHitBoundary = jest.fn();
      const interleavedLogs = logs.map(toInterleavedLog);
      const selectedLog = interleavedLogs[1]; // req1, timestamp 3000

      const rightPanelRef = createRef<HTMLDivElement>();

      renderHook(() =>
        useNavigateLogs(
          selectedLog,
          interleavedLogs,
          onSelectLog,
          jest.fn(),
          onHitBoundary,
          rightPanelRef,
        ),
      );

      // Trigger the shift+down hotkey
      act(() => {
        mockHotkeys["shift+down"]?.();
      });

      // Should select the next log in the same request (req1, timestamp 2000)
      expect(onSelectLog).toHaveBeenCalledWith(interleavedLogs[2]);
      expect(onHitBoundary).toHaveBeenCalledWith(null);
    });

    it("should navigate only within the same request with shift+up", () => {
      const logs: UdfLog[] = [
        createLog(4000, "req2", "exec2"),
        createLog(3000, "req1", "exec1"),
        createLog(2000, "req1", "exec1"),
        createLog(1000, "req2", "exec2"),
      ];
      const onSelectLog = jest.fn();
      const onHitBoundary = jest.fn();
      const interleavedLogs = logs.map(toInterleavedLog);
      const selectedLog = interleavedLogs[2]; // req1, timestamp 2000

      const rightPanelRef = createRef<HTMLDivElement>();

      renderHook(() =>
        useNavigateLogs(
          selectedLog,
          interleavedLogs,
          onSelectLog,
          jest.fn(),
          onHitBoundary,
          rightPanelRef,
        ),
      );

      // Trigger the shift+up hotkey
      act(() => {
        mockHotkeys["shift+up"]?.();
      });

      // Should select the previous log in the same request (req1, timestamp 3000)
      expect(onSelectLog).toHaveBeenCalledWith(interleavedLogs[1]);
      expect(onHitBoundary).toHaveBeenCalledWith(null);
    });

    it("should hit boundary when at the end of request scope", () => {
      const logs: UdfLog[] = [
        createLog(4000, "req2", "exec2"),
        createLog(3000, "req1", "exec1"),
        createLog(2000, "req1", "exec1"),
        createLog(1000, "req2", "exec2"),
      ];
      const onSelectLog = jest.fn();
      const onHitBoundary = jest.fn();
      const interleavedLogs = logs.map(toInterleavedLog);
      const selectedLog = interleavedLogs[2]; // req1, timestamp 2000 (last in request)

      const rightPanelRef = createRef<HTMLDivElement>();

      renderHook(() =>
        useNavigateLogs(
          selectedLog,
          interleavedLogs,
          onSelectLog,
          jest.fn(),
          onHitBoundary,
          rightPanelRef,
        ),
      );

      // Trigger the shift+down hotkey
      act(() => {
        mockHotkeys["shift+down"]?.();
      });

      // Should hit the bottom boundary
      expect(onSelectLog).not.toHaveBeenCalled();
      expect(onHitBoundary).toHaveBeenCalledWith("bottom");
    });
  });

  describe("navigation within execution scope", () => {
    it("should navigate only within the same execution with ctrl/meta+down", () => {
      const logs: UdfLog[] = [
        createLog(4000, "req1", "exec2"),
        createLog(3000, "req1", "exec1"),
        createLog(2000, "req1", "exec1"),
        createLog(1000, "req1", "exec2"),
      ];
      const onSelectLog = jest.fn();
      const onHitBoundary = jest.fn();
      const interleavedLogs = logs.map(toInterleavedLog);
      const selectedLog = interleavedLogs[1]; // exec1, timestamp 3000

      const rightPanelRef = createRef<HTMLDivElement>();

      renderHook(() =>
        useNavigateLogs(
          selectedLog,
          interleavedLogs,
          onSelectLog,
          jest.fn(),
          onHitBoundary,
          rightPanelRef,
        ),
      );

      // Trigger the ctrl+down hotkey
      act(() => {
        mockHotkeys["ctrl+down"]?.();
      });

      // Should select the next log in the same execution (exec1, timestamp 2000)
      expect(onSelectLog).toHaveBeenCalledWith(interleavedLogs[2]);
      expect(onHitBoundary).toHaveBeenCalledWith(null);
    });

    it("should navigate only within the same execution with ctrl/meta+up", () => {
      const logs: UdfLog[] = [
        createLog(4000, "req1", "exec2"),
        createLog(3000, "req1", "exec1"),
        createLog(2000, "req1", "exec1"),
        createLog(1000, "req1", "exec2"),
      ];
      const onSelectLog = jest.fn();
      const onHitBoundary = jest.fn();
      const interleavedLogs = logs.map(toInterleavedLog);
      const selectedLog = interleavedLogs[2]; // exec1, timestamp 2000

      const rightPanelRef = createRef<HTMLDivElement>();

      renderHook(() =>
        useNavigateLogs(
          selectedLog,
          interleavedLogs,
          onSelectLog,
          jest.fn(),
          onHitBoundary,
          rightPanelRef,
        ),
      );

      // Trigger the meta+up hotkey
      act(() => {
        mockHotkeys["meta+up"]?.();
      });

      // Should select the previous log in the same execution (exec1, timestamp 3000)
      expect(onSelectLog).toHaveBeenCalledWith(interleavedLogs[1]);
      expect(onHitBoundary).toHaveBeenCalledWith(null);
    });

    it("should hit boundary when at the end of execution scope", () => {
      const logs: UdfLog[] = [
        createLog(4000, "req1", "exec2"),
        createLog(3000, "req1", "exec1"),
        createLog(2000, "req1", "exec1"),
        createLog(1000, "req1", "exec2"),
      ];
      const onSelectLog = jest.fn();
      const onHitBoundary = jest.fn();
      const interleavedLogs = logs.map(toInterleavedLog);
      const selectedLog = interleavedLogs[2]; // exec1, timestamp 2000 (last in execution)

      const rightPanelRef = createRef<HTMLDivElement>();

      renderHook(() =>
        useNavigateLogs(
          selectedLog,
          interleavedLogs,
          onSelectLog,
          jest.fn(),
          onHitBoundary,
          rightPanelRef,
        ),
      );

      // Trigger the ctrl+down hotkey
      act(() => {
        mockHotkeys["ctrl+down"]?.();
      });

      // Should hit the bottom boundary
      expect(onSelectLog).not.toHaveBeenCalled();
      expect(onHitBoundary).toHaveBeenCalledWith("bottom");
    });
  });

  describe("edge cases", () => {
    it("should handle logs with unsorted timestamps", () => {
      const logs: UdfLog[] = [
        createLog(2000, "req1", "exec1"),
        createLog(3000, "req1", "exec1"), // Out of order
        createLog(1000, "req1", "exec1"),
      ];
      const onSelectLog = jest.fn();
      const onHitBoundary = jest.fn();
      const interleavedLogs = logs.map(toInterleavedLog);
      const selectedLog = interleavedLogs[0]; // timestamp 2000

      const rightPanelRef = createRef<HTMLDivElement>();

      renderHook(() =>
        useNavigateLogs(
          selectedLog,
          interleavedLogs,
          onSelectLog,
          jest.fn(),
          onHitBoundary,
          rightPanelRef,
        ),
      );

      // Trigger the down arrow hotkey
      act(() => {
        mockHotkeys.down?.();
      });

      // Should select the next log in array order (not sorted by timestamp)
      // Navigation works on the array as-is; logs should be pre-sorted in production
      expect(onSelectLog).toHaveBeenCalledWith(interleavedLogs[1]);
      expect(onHitBoundary).toHaveBeenCalledWith(null);
    });

    it("should handle when selectedLog is null", () => {
      const logs: UdfLog[] = [
        createLog(3000, "req1", "exec1"),
        createLog(2000, "req1", "exec1"),
      ];
      const interleavedLogs = logs.map(toInterleavedLog);
      const onSelectLog = jest.fn();
      const onHitBoundary = jest.fn();
      const rightPanelRef = createRef<HTMLDivElement>();

      renderHook(() =>
        useNavigateLogs(
          null,
          interleavedLogs,
          onSelectLog,
          jest.fn(),
          onHitBoundary,
          rightPanelRef,
        ),
      );

      // Trigger the down arrow hotkey
      act(() => {
        mockHotkeys.down?.();
      });

      // Should not call any callbacks when selectedLog is null
      expect(onSelectLog).not.toHaveBeenCalled();
      expect(onHitBoundary).not.toHaveBeenCalled();
    });

    it("should handle single log in the list", () => {
      const logs: UdfLog[] = [createLog(1000, "req1", "exec1")];
      const onSelectLog = jest.fn();
      const onHitBoundary = jest.fn();
      const interleavedLogs = logs.map(toInterleavedLog);
      const selectedLog = interleavedLogs[0];

      const rightPanelRef = createRef<HTMLDivElement>();

      renderHook(() =>
        useNavigateLogs(
          selectedLog,
          interleavedLogs,
          onSelectLog,
          jest.fn(),
          onHitBoundary,
          rightPanelRef,
        ),
      );

      // Try to navigate down
      act(() => {
        mockHotkeys.down?.();
      });

      // Should hit the bottom boundary
      expect(onSelectLog).not.toHaveBeenCalled();
      expect(onHitBoundary).toHaveBeenCalledWith("bottom");
    });

    it("should handle mixed log types (log and outcome)", () => {
      const logs: UdfLog[] = [
        createLog(3000, "req1", "exec1", "outcome"),
        createLog(2000, "req1", "exec1", "log"),
        createLog(1000, "req1", "exec1", "log"),
      ];
      const onSelectLog = jest.fn();
      const onHitBoundary = jest.fn();
      const interleavedLogs = logs.map(toInterleavedLog);
      const selectedLog = interleavedLogs[1]; // log type, timestamp 2000

      const rightPanelRef = createRef<HTMLDivElement>();

      renderHook(() =>
        useNavigateLogs(
          selectedLog,
          interleavedLogs,
          onSelectLog,
          jest.fn(),
          onHitBoundary,
          rightPanelRef,
        ),
      );

      // Trigger the down arrow hotkey
      act(() => {
        mockHotkeys.down?.();
      });

      // Should navigate to the next log regardless of type
      expect(onSelectLog).toHaveBeenCalledWith(interleavedLogs[2]);
      expect(onHitBoundary).toHaveBeenCalledWith(null);
    });
  });
});
