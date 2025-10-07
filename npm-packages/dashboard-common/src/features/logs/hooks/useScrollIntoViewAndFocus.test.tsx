import { renderHook } from "@testing-library/react";
import { useScrollIntoViewAndFocus } from "./useScrollIntoViewAndFocus";

describe("useScrollIntoViewAndFocus", () => {
  let mockScrollIntoView: jest.Mock;
  let mockFocus: jest.Mock;

  beforeEach(() => {
    mockScrollIntoView = jest.fn();
    mockFocus = jest.fn();
  });

  it("should not focus or scroll when focused is false", () => {
    const { result } = renderHook(() =>
      useScrollIntoViewAndFocus({ focused: false }),
    );

    // @ts-expect-error - mock element for testing
    result.current.elementRef.current = {
      scrollIntoView: mockScrollIntoView,
    };
    // @ts-expect-error - mock button for testing
    result.current.buttonRef.current = {
      focus: mockFocus,
    };

    expect(mockFocus).not.toHaveBeenCalled();
    expect(mockScrollIntoView).not.toHaveBeenCalled();
  });

  it("should focus and scroll when focused changes to true", () => {
    const { result, rerender } = renderHook(
      ({ focused }) => useScrollIntoViewAndFocus({ focused }),
      { initialProps: { focused: false } },
    );

    // Set up refs before changing focused state
    // @ts-expect-error - mock element for testing
    result.current.elementRef.current = {
      scrollIntoView: mockScrollIntoView,
    };
    // @ts-expect-error - mock button for testing
    result.current.buttonRef.current = {
      focus: mockFocus,
    };

    // Change focused to true
    rerender({ focused: true });

    expect(mockFocus).toHaveBeenCalledTimes(1);
    expect(mockScrollIntoView).toHaveBeenCalledTimes(1);
    expect(mockScrollIntoView).toHaveBeenCalledWith({
      block: "nearest",
      inline: "nearest",
    });
  });

  it("should focus and scroll on initial render when focused is true, but not on subsequent rerenders", () => {
    // Start with focused=false to set up refs first
    const { result, rerender } = renderHook(
      ({ focused }) => useScrollIntoViewAndFocus({ focused }),
      { initialProps: { focused: false } },
    );

    // Set up refs before transitioning to focused
    // @ts-expect-error - mock element for testing
    result.current.elementRef.current = {
      scrollIntoView: mockScrollIntoView,
    };
    // @ts-expect-error - mock button for testing
    result.current.buttonRef.current = {
      focus: mockFocus,
    };

    // Transition to focused=true
    rerender({ focused: true });

    // Should have focused and scrolled on transition
    expect(mockFocus).toHaveBeenCalledTimes(1);
    expect(mockScrollIntoView).toHaveBeenCalledTimes(1);

    // Rerender with focused still true
    mockFocus.mockClear();
    mockScrollIntoView.mockClear();
    rerender({ focused: true });

    // Should not focus or scroll again (only on transition to focused)
    expect(mockFocus).not.toHaveBeenCalled();
    expect(mockScrollIntoView).not.toHaveBeenCalled();
  });

  it("should not focus or scroll when enabled is false", () => {
    const { result, rerender } = renderHook(
      ({ focused, enabled }) => useScrollIntoViewAndFocus({ focused, enabled }),
      { initialProps: { focused: false, enabled: false } },
    );

    // @ts-expect-error - mock element for testing
    result.current.elementRef.current = {
      scrollIntoView: mockScrollIntoView,
    };
    // @ts-expect-error - mock button for testing
    result.current.buttonRef.current = {
      focus: mockFocus,
    };

    // Change focused to true while enabled is false
    rerender({ focused: true, enabled: false });

    expect(mockFocus).not.toHaveBeenCalled();
    expect(mockScrollIntoView).not.toHaveBeenCalled();
  });

  it("should focus and scroll when both focused and enabled become true", () => {
    const { result, rerender } = renderHook(
      ({ focused, enabled }) => useScrollIntoViewAndFocus({ focused, enabled }),
      { initialProps: { focused: false, enabled: false } },
    );

    // @ts-expect-error - mock element for testing
    result.current.elementRef.current = {
      scrollIntoView: mockScrollIntoView,
    };
    // @ts-expect-error - mock button for testing
    result.current.buttonRef.current = {
      focus: mockFocus,
    };

    // Change both to true
    rerender({ focused: true, enabled: true });

    expect(mockFocus).toHaveBeenCalledTimes(1);
    expect(mockScrollIntoView).toHaveBeenCalledTimes(1);
  });

  it("should handle missing element ref gracefully", () => {
    const { result, rerender } = renderHook(
      ({ focused }) => useScrollIntoViewAndFocus({ focused }),
      { initialProps: { focused: false } },
    );

    // @ts-expect-error - mock button for testing
    result.current.buttonRef.current = {
      focus: mockFocus,
    };
    // Don't set elementRef.current

    rerender({ focused: true });

    // Should still focus button
    expect(mockFocus).toHaveBeenCalledTimes(1);
    // Should not throw error when trying to scroll
    expect(mockScrollIntoView).not.toHaveBeenCalled();
  });

  it("should handle missing button ref gracefully", () => {
    const { result, rerender } = renderHook(
      ({ focused }) => useScrollIntoViewAndFocus({ focused }),
      { initialProps: { focused: false } },
    );

    // @ts-expect-error - mock element for testing
    result.current.elementRef.current = {
      scrollIntoView: mockScrollIntoView,
    };
    // Don't set buttonRef.current

    rerender({ focused: true });

    // Should still scroll element
    expect(mockScrollIntoView).toHaveBeenCalledTimes(1);
    // Should not throw error when trying to focus
    expect(mockFocus).not.toHaveBeenCalled();
  });

  it("should scroll again when transitioning from false to true multiple times", () => {
    const { result, rerender } = renderHook(
      ({ focused }) => useScrollIntoViewAndFocus({ focused }),
      { initialProps: { focused: false } },
    );

    // @ts-expect-error - mock element for testing
    result.current.elementRef.current = {
      scrollIntoView: mockScrollIntoView,
    };
    // @ts-expect-error - mock button for testing
    result.current.buttonRef.current = {
      focus: mockFocus,
    };

    // First transition to true
    rerender({ focused: true });
    expect(mockScrollIntoView).toHaveBeenCalledTimes(1);

    // Transition to false
    mockScrollIntoView.mockClear();
    rerender({ focused: false });
    expect(mockScrollIntoView).not.toHaveBeenCalled();

    // Second transition to true
    rerender({ focused: true });
    expect(mockScrollIntoView).toHaveBeenCalledTimes(1);
  });
});
