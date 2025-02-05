import React from "react";
import { renderHook } from "@testing-library/react";
import { useIsOverflowing } from "@common/lib/useIsOverflowing";

type FakeElement = {
  scrollWidth: number;
  clientWidth: number;
};

test("should run again even if ref identity does not change", () => {
  const fakeElement = {
    scrollWidth: 30,
    clientWidth: 20,
  };
  const ref =
    React.createRef<HTMLElement>() as React.MutableRefObject<HTMLElement>;
  ref.current = fakeElement as HTMLElement;
  const { result, rerender } = renderHook(() => useIsOverflowing(ref));
  expect(result.current).toBe(true);

  (ref.current as FakeElement).scrollWidth = 10;

  rerender();

  expect(result.current).toBe(false);
});
