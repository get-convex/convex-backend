import { act, renderHook } from "@testing-library/react";
import { RefObject } from "react";
import { FixedSizeList } from "react-window";
import { useStickyLogs } from "./useStickyLogs";
import { InterleavedLog } from "../utils/interleaveLogs";

const mockLogs = [
  {
    kind: "ExecutionLog",
    executionLog: {
      timestamp: 1,
      outcome: {
        status: "success",
      },
    },
  },
  {
    kind: "ExecutionLog",
    executionLog: {
      timestamp: 2,
      outcome: {
        status: "success",
      },
    },
  },
  {
    kind: "ExecutionLog",
    executionLog: {
      timestamp: 3,
      outcome: {
        status: "success",
      },
    },
  },
] as InterleavedLog[];

describe("useStickyLogs", () => {
  const mockScrollThreshold = 100;
  const mockListRef = {
    current: {
      resetAfterIndex: jest.fn(),
      scrollToItem: jest.fn(),
    },
  } as unknown as RefObject<FixedSizeList>;

  beforeEach(() => {
    jest.resetAllMocks();
  });

  it("should scroll to the bottom when showNewLogs hook is called", () => {
    const { result, rerender } = renderHook(
      ({
        logs,
        listRef,
        scrollThreshold,
      }: {
        logs: InterleavedLog[];
        listRef: RefObject<FixedSizeList>;
        scrollThreshold: number;
      }) => useStickyLogs(listRef, logs, scrollThreshold),
      {
        initialProps: {
          logs: mockLogs,
          listRef: mockListRef,
          scrollThreshold: mockScrollThreshold,
        },
      },
    );

    // Should scroll to the bottom once on init.
    expect(mockListRef.current?.scrollToItem).toHaveBeenCalledTimes(1);

    expect(result.current.showNewLogs).toBeNull();

    act(() => {
      // Scroll up by 1 px
      result.current.onScroll({
        scrollOffset: mockScrollThreshold - 1,
        scrollDirection: "backward",
        scrollUpdateWasRequested: false,
      });
    });

    const newLogs = [
      ...mockLogs,
      {
        kind: "ExecutionLog",
        executionLog: {
          timestamp: 4,
          outcome: {
            status: "success",
          },
        },
      },
    ] as InterleavedLog[];

    const newNewProps = {
      listRef: mockListRef,
      scrollThreshold: mockScrollThreshold,
      logs: newLogs,
    };

    rerender(newNewProps);
    expect(result.current.showNewLogs).not.toBeNull();
    act(() => {
      result.current?.showNewLogs!();
    });

    expect(mockListRef.current?.scrollToItem).toHaveBeenCalledTimes(2);
    expect(mockListRef.current?.scrollToItem).toHaveBeenLastCalledWith(
      4,
      "end",
    );
  });

  it("should scroll to the bottom when new logs are added", () => {
    const { result, rerender } = renderHook(
      ({
        logs,
        listRef,
        scrollThreshold,
      }: {
        logs: InterleavedLog[];
        listRef: RefObject<FixedSizeList>;
        scrollThreshold: number;
      }) => useStickyLogs(listRef, logs, scrollThreshold),
      {
        initialProps: {
          logs: mockLogs,
          listRef: mockListRef,
          scrollThreshold: mockScrollThreshold,
        },
      },
    );

    // Should scroll to the bottom once on init.
    expect(mockListRef.current?.scrollToItem).toHaveBeenCalledTimes(1);
    expect(result.current.showNewLogs).toBeNull();

    const newLogs = [
      ...mockLogs,
      {
        kind: "ExecutionLog",
        executionLog: {
          timestamp: 4,
          outcome: {
            status: "success",
          },
        },
      },
    ] as InterleavedLog[];

    rerender({
      logs: newLogs,
      listRef: mockListRef,
      scrollThreshold: mockScrollThreshold,
    });

    expect(mockListRef.current?.scrollToItem).toHaveBeenCalledTimes(2);
    expect(mockListRef.current?.scrollToItem).toHaveBeenLastCalledWith(
      newLogs.length,
      "end",
    );
  });

  it("should not scroll to the bottom when new logs are added and the user has scrolled up", () => {
    const { result, rerender } = renderHook(
      ({
        logs,
        listRef,
        scrollThreshold,
      }: {
        logs: InterleavedLog[];
        listRef: RefObject<FixedSizeList>;
        scrollThreshold: number;
      }) => useStickyLogs(listRef, logs, scrollThreshold),
      {
        initialProps: {
          logs: mockLogs,
          listRef: mockListRef,
          scrollThreshold: mockScrollThreshold,
        },
      },
    );

    // Should scroll to the bottom once on init.
    expect(mockListRef.current?.scrollToItem).toHaveBeenCalledTimes(1);
    expect(result.current.showNewLogs).toBeNull();

    act(() => {
      // Scroll up by 1 px
      result.current.onScroll({
        scrollOffset: mockScrollThreshold - 1,
        scrollDirection: "backward",
        scrollUpdateWasRequested: false,
      });
    });

    const newLogs = [
      ...mockLogs,
      {
        kind: "ExecutionLog",
        executionLog: {
          timestamp: 4,
          outcome: {
            status: "success",
          },
        },
      },
    ] as InterleavedLog[];

    rerender({
      logs: newLogs,
      listRef: mockListRef,
      scrollThreshold: mockScrollThreshold,
    });

    expect(mockListRef.current?.scrollToItem).toHaveBeenCalledTimes(1);
  });
});
