import { renderHook, act } from "@testing-library/react";
import { useHitBoundary } from "./LogList";

describe("useHitBoundary", () => {
  beforeEach(() => {
    jest.useFakeTimers();
  });

  afterEach(() => {
    jest.runOnlyPendingTimers();
    jest.useRealTimers();
  });

  it("should initialize with null boundary", () => {
    const { result } = renderHook(() => useHitBoundary());

    expect(result.current.hitBoundary).toBeNull();
  });

  it("should set boundary when called", () => {
    const { result } = renderHook(() => useHitBoundary());

    act(() => {
      result.current.setHitBoundary("top");
    });

    expect(result.current.hitBoundary).toBe("top");
  });

  it("should automatically reset boundary to null after 750ms", () => {
    const { result } = renderHook(() => useHitBoundary());

    act(() => {
      result.current.setHitBoundary("bottom");
    });

    expect(result.current.hitBoundary).toBe("bottom");

    // Fast-forward time by 750ms
    act(() => {
      jest.advanceTimersByTime(750);
    });

    expect(result.current.hitBoundary).toBeNull();
  });

  it("should clear previous timeout when setting a new boundary", () => {
    const { result } = renderHook(() => useHitBoundary());

    // Set first boundary
    act(() => {
      result.current.setHitBoundary("top");
    });

    expect(result.current.hitBoundary).toBe("top");

    // Fast-forward time by 400ms (not enough to trigger reset)
    act(() => {
      jest.advanceTimersByTime(400);
    });

    expect(result.current.hitBoundary).toBe("top");

    // Set second boundary (should clear the previous timeout)
    act(() => {
      result.current.setHitBoundary("bottom");
    });

    expect(result.current.hitBoundary).toBe("bottom");

    // Fast-forward time by 400ms (total 800ms from first set, but only 400ms from second)
    act(() => {
      jest.advanceTimersByTime(400);
    });

    // Should still be "bottom" because the timer was reset
    expect(result.current.hitBoundary).toBe("bottom");

    // Fast-forward remaining 350ms
    act(() => {
      jest.advanceTimersByTime(350);
    });

    // Now it should be null
    expect(result.current.hitBoundary).toBeNull();
  });

  it("should allow manual reset to null without timeout", () => {
    const { result } = renderHook(() => useHitBoundary());

    act(() => {
      result.current.setHitBoundary("top");
    });

    expect(result.current.hitBoundary).toBe("top");

    // Manually reset to null
    act(() => {
      result.current.setHitBoundary(null);
    });

    expect(result.current.hitBoundary).toBeNull();

    // Fast-forward time to ensure no timer is running
    act(() => {
      jest.advanceTimersByTime(1000);
    });

    // Should still be null
    expect(result.current.hitBoundary).toBeNull();
  });

  it("should cleanup timeout on unmount", () => {
    const { result, unmount } = renderHook(() => useHitBoundary());

    act(() => {
      result.current.setHitBoundary("bottom");
    });

    expect(result.current.hitBoundary).toBe("bottom");

    // Unmount before timer completes
    unmount();

    // Fast-forward time
    act(() => {
      jest.advanceTimersByTime(1000);
    });

    // No errors should occur from the timer trying to update state after unmount
  });

  it("should handle rapid boundary changes", () => {
    const { result } = renderHook(() => useHitBoundary());

    // Set boundary multiple times rapidly
    act(() => {
      result.current.setHitBoundary("top");
      result.current.setHitBoundary("bottom");
      result.current.setHitBoundary("top");
    });

    expect(result.current.hitBoundary).toBe("top");

    // Only the last timeout should be active
    act(() => {
      jest.advanceTimersByTime(750);
    });

    expect(result.current.hitBoundary).toBeNull();
  });
});
