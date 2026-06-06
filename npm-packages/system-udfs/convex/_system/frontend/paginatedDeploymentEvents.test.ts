import { describe, expect, test, vi } from "vitest";
import { clampForAuditLogRetention } from "./auditLogRetention";

function dbWithAuditLogRetentionDays(auditLogRetentionDays: number) {
  return {
    query: (tableName: string) => {
      expect(tableName).toBe("_backend_info");
      return {
        first: async () => ({ auditLogRetentionDays }),
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
});
