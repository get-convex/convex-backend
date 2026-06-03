import React from "react";
import { webcrypto } from "crypto";
import { TextEncoder } from "util";
import "@testing-library/jest-dom";
import { mockAnimationsApi } from "jsdom-testing-mocks";

// jsdom does not expose Web Crypto or TextEncoder; polyfill from Node so
// browser-only modules (e.g. PKCE helpers) work under Jest.
if (!globalThis.crypto?.subtle) {
  Object.defineProperty(globalThis, "crypto", {
    value: webcrypto,
    configurable: true,
  });
}
if (typeof globalThis.TextEncoder === "undefined") {
  // @ts-expect-error — Node's TextEncoder is structurally compatible.
  globalThis.TextEncoder = TextEncoder;
}

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
