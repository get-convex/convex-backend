import React from "react";
import "@testing-library/jest-dom";
import { mockAnimationsApi } from "jsdom-testing-mocks";

global.ResizeObserver = function MockResizeObserver() {
  return {
    observe: jest.fn(),
    unobserve: jest.fn(),
    disconnect: jest.fn(),
  };
} as any;

mockAnimationsApi();

// Mock Transition from @headlessui/react to avoid waiting for the transition to complete in tests
jest.mock("@headlessui/react", () => ({
  ...jest.requireActual("@headlessui/react"),
  Transition: ({
    children,
    show,
    afterLeave,
  }: React.PropsWithChildren<{
    show: boolean;
    afterLeave?: () => void;
  }>) => {
    const prevShow = React.useRef<boolean | undefined>(undefined);
    React.useEffect(() => {
      if (prevShow.current && !show) {
        afterLeave?.();
      }
      prevShow.current = show;
    }, [show, afterLeave]);

    return show ? children : null;
  },
  TransitionChild: ({ children }: React.PropsWithChildren) => children,
}));
