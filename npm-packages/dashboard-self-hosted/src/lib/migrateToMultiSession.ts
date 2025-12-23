import { Session } from "./useMultiSession";

const OLD_KEYS = {
  adminKey: "adminKey",
  deploymentUrl: "deploymentUrl",
  deploymentName: "deploymentName",
};

const SESSIONS_STORAGE_KEY = "convex-dashboard-sessions";
const MIGRATION_FLAG_KEY = "convex-multi-session-migrated";

/**
 * Migrates from old single-session localStorage format to new multi-session format
 * Only runs once per browser
 */
export function migrateToMultiSession(): void {
  // Check if already migrated
  if (localStorage.getItem(MIGRATION_FLAG_KEY) === "true") {
    return;
  }

  try {
    // Check if old format exists
    const oldAdminKey = localStorage.getItem(OLD_KEYS.adminKey);
    const oldDeploymentUrl = localStorage.getItem(OLD_KEYS.deploymentUrl);
    const oldDeploymentName = localStorage.getItem(OLD_KEYS.deploymentName);

    // If no old data exists, just mark as migrated
    if (!oldAdminKey && !oldDeploymentUrl) {
      localStorage.setItem(MIGRATION_FLAG_KEY, "true");
      return;
    }

    // Check if new format already exists
    const existingData = localStorage.getItem(SESSIONS_STORAGE_KEY);
    if (existingData) {
      // New format exists, just clean up old keys and mark migrated
      localStorage.removeItem(OLD_KEYS.adminKey);
      localStorage.removeItem(OLD_KEYS.deploymentUrl);
      localStorage.removeItem(OLD_KEYS.deploymentName);
      localStorage.setItem(MIGRATION_FLAG_KEY, "true");
      return;
    }

    // Migrate old data to new format
    if (oldAdminKey && oldDeploymentUrl) {
      const now = Date.now();
      const session: Session = {
        id: `session-migrated-${now}`,
        name: "Migrated Session",
        deploymentUrl: oldDeploymentUrl,
        adminKey: oldAdminKey,
        deploymentName: oldDeploymentName || "",
        lastAccessed: now,
        createdAt: now,
      };

      const newData = {
        sessions: [session],
        activeSessionId: session.id,
      };

      localStorage.setItem(SESSIONS_STORAGE_KEY, JSON.stringify(newData));

      // Clean up old keys
      localStorage.removeItem(OLD_KEYS.adminKey);
      localStorage.removeItem(OLD_KEYS.deploymentUrl);
      localStorage.removeItem(OLD_KEYS.deploymentName);
    }

    localStorage.setItem(MIGRATION_FLAG_KEY, "true");
  } catch (e) {
    console.error("Failed to migrate to multi-session format:", e);
    // Don't throw - better to continue without migration than to break
  }
}
