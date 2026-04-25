import "@testing-library/jest-dom";

// Headless UI's Menu (used by Menu/MenuItem) requires ResizeObserver to be
// present even in jsdom. jsdom doesn't ship it, so polyfill with a no-op.
class ResizeObserverStub {
  observe() {}
  unobserve() {}
  disconnect() {}
}
(globalThis as unknown as { ResizeObserver: typeof ResizeObserverStub }).ResizeObserver =
  ResizeObserverStub;
