import { beforeEach, describe, expect, test, vi } from "vitest";

vi.mock("./syscall.js", () => ({
  performAsyncSyscall: vi.fn(),
}));

import { performAsyncSyscall } from "./syscall.js";
import { setupQueryMeta } from "./meta_impl.js";

describe("setupQueryMeta", () => {
  beforeEach(() => {
    vi.mocked(performAsyncSyscall).mockReset();
  });

  test("reads invocation context through the syscall bridge", async () => {
    const invocationContext = {
      requestId: "req_123",
      executionId: "exec_123",
      isRoot: true,
      parentScheduledJob: null,
      parentScheduledJobComponentId: null,
      metadata: {
        correlationId: "corr_123",
        origin: "nuxt",
      },
    };
    vi.mocked(performAsyncSyscall).mockResolvedValue(invocationContext);

    const context = await setupQueryMeta("public").getInvocationContext();

    expect(performAsyncSyscall).toHaveBeenCalledWith(
      "1.0/getInvocationContext",
      {},
    );
    expect(context).toEqual(invocationContext);
  });
});
