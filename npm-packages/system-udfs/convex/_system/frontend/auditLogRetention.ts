import type { DatabaseReader } from "../../_generated/server";

const MS_PER_DAY = 24 * 60 * 60 * 1000;

export async function clampForAuditLogRetention(
  db: DatabaseReader,
  minDate: number,
) {
  const backendInfo = await db.query("_backend_info").first();
  if (backendInfo?.auditLogRetentionDays == null) {
    return minDate;
  }
  const auditLogRetentionDays = Number(backendInfo.auditLogRetentionDays);
  // no limit if auditLogRetentionDays is -1
  if (auditLogRetentionDays === -1) {
    return minDate;
  }
  const minAllowable = startOfUtcDay(
    Date.now() - (auditLogRetentionDays + 1) * MS_PER_DAY,
  );
  if (minDate < minAllowable) {
    return minAllowable;
  }
  return minDate;
}

function startOfUtcDay(timestamp: number) {
  const date = new Date(timestamp);
  date.setUTCHours(0, 0, 0, 0);
  return date.getTime();
}
