import type { HostCapacity } from "../../lib/orchestratorApi";
import { capacityProjection } from "./CapacityStrip";

const capacity: HostCapacity = {
  totalMemoryMb: 122.8 * 1024,
  totalCpus: 32,
  allocatedMemoryMb: 64 * 1024,
  allocatedCpus: 32,
  deploymentCount: 1,
};

describe("capacity projection", () => {
  test("reports projected host usage after replacing the current deployment tier", () => {
    expect(capacityProjection(capacity, "S256", "S256")).toMatchObject({
      baselineAllocatedMb: 0,
      projectedAllocatedMb: 64 * 1024,
      baselinePct: 0,
      projectedPct: 52,
      projectedBarPct: 52,
      over: false,
      warn: false,
    });
  });

  test("marks projected allocations over host capacity", () => {
    expect(capacityProjection(capacity, "custom:131072:64")).toMatchObject({
      baselineAllocatedMb: 64 * 1024,
      projectedAllocatedMb: 192 * 1024,
      baselinePct: 52,
      projectedPct: 156,
      projectedBarPct: 100,
      over: true,
      warn: false,
    });
  });
});
