import { describe, expect, test, vi } from "vitest";
import { clampForAuditLogRetention } from "./auditLogRetention";
import { hasDeploymentEventPostFilters } from "./deploymentEventFilters";

function dbWithAuditLogRetentionDays(auditLogRetentionDays: number | null) {
  return {
    query: (tableName: string) => {
      expect(tableName).toBe("_backend_info");
      return {
        first: async () =>
          auditLogRetentionDays === null ? null : { auditLogRetentionDays },
      };
    },
  };
}

describe("clampForAuditLogRetention", () => {
  test("uses a stable retention floor during the same day", async () => {
    const referenceTime = Date.UTC(2026, 0, 15, 12, 0, 0);
    const requestedMinDate = Date.UTC(2025, 0, 1);
    const dateNow = vi.spyOn(Date, "now");
    dateNow
      .mockReturnValueOnce(referenceTime)
      .mockReturnValueOnce(referenceTime + 60_000);

    const firstClampedMinDate = await clampForAuditLogRetention(
      dbWithAuditLogRetentionDays(90) as any,
      requestedMinDate,
    );
    const secondClampedMinDate = await clampForAuditLogRetention(
      dbWithAuditLogRetentionDays(90) as any,
      requestedMinDate,
    );

    expect(firstClampedMinDate).toBe(secondClampedMinDate);
    expect(firstClampedMinDate).toBe(Date.UTC(2025, 9, 16));
  });

  test("does not clamp when backend info is missing for self-hosted deployments", async () => {
    const requestedMinDate = Date.UTC(2023, 0, 1);

    const clampedMinDate = await clampForAuditLogRetention(
      dbWithAuditLogRetentionDays(null) as any,
      requestedMinDate,
    );

    expect(clampedMinDate).toBe(requestedMinDate);
  });
});

describe("hasDeploymentEventPostFilters", () => {
  test("skips the post-filter for date-only history queries", () => {
    expect(hasDeploymentEventPostFilters({})).toBe(false);
  });

  test("keeps explicit post-filters for author and action filters", () => {
    expect(hasDeploymentEventPostFilters({ authorMemberIds: [] })).toBe(true);
    expect(hasDeploymentEventPostFilters({ actions: [] })).toBe(true);
    expect(
      hasDeploymentEventPostFilters({
        authorMemberIds: [BigInt(1)],
        actions: ["push_config"],
      }),
    ).toBe(true);
  });
});
