import { Config, nowSeconds, ProvisionerInfo, Scenario } from "../scenario";
import { EXPORT_TIMEOUT } from "../types";
import { ScenarioError } from "../metrics";
import { ConvexClient } from "convex/browser";

type Headers = {
  Authorization: string;
};

export class CloudBackup extends Scenario implements Scenario {
  provisionerInfo: ProvisionerInfo;
  headers: Headers;
  constructor(config: Config) {
    super("CloudBackup", config);
    if (!config.provisionerInfo) {
      throw new Error("CloudBackup job only works with Big Brain available");
    }
    this.provisionerInfo = config.provisionerInfo;
    this.headers = {
      Authorization: `Bearer ${config.provisionerInfo.accessToken}`,
    };
  }

  async run(_client: ConvexClient) {
    const t0 = nowSeconds();
    const res = await fetch(
      `${this.provisionerInfo.provisionHost}/api/dashboard/deployments/${this.provisionerInfo.deploymentId}/request_cloud_backup`,
      {
        method: "POST",
        headers: this.headers,
      },
    );
    if (res.status === 200) {
      this.sendCountMetric(1, "request_backup_succeeded");
    } else {
      const data = await res.text();
      this.sendError(data, "request_backup_failed");
      throw new Error("Request backup failed");
    }
    const backup = await res.json();

    try {
      await this.executeOrTimeoutWithLatency(
        this.waitForBackup(backup.id),
        EXPORT_TIMEOUT,
        "backup_timeout",
        "backup",
        t0,
      );
    } catch (err: any) {
      this.sendError(err, "backup_failed");
    }
  }

  async waitForBackup(cloudBackupId: number) {
    try {
      await this.waitForBackupInner(cloudBackupId);
      this.sendCountMetric(1, "backup_completed");
    } catch (err: any) {
      this.sendError(err, "backup_failed");
    }
  }

  async waitForBackupInner(cloudBackupId: number) {
    // Watch for completion
    for (let numRetries = 0; numRetries < Infinity; numRetries++) {
      const res = await fetch(
        `${this.provisionerInfo.provisionHost}/api/dashboard/cloud_backups/${cloudBackupId}`,
        {
          headers: this.headers,
        },
      );
      if (res.status === 200) {
        const response = await res.json();
        switch (response.state) {
          case "complete":
            return;
          case "requested":
          case "inProgress":
            break;
          case "failed":
            throw new Error("Backup failed");
          default:
            throw new Error(`Unknown state ${response.state}`);
        }
      } else {
        const data = await res.text();
        this.sendError(data, "get_backup_failed");
        throw new Error("Get backup failed");
      }

      await new Promise((resolve) =>
        setTimeout(resolve, backoffWithJitter(numRetries)),
      );
    }
  }

  defaultErrorName(): ScenarioError {
    return "backup_failure";
  }
}

// Backoff numbers are in milliseconds.
const INITIAL_BACKOFF = 500;
const MAX_BACKOFF = 16000;

const backoffWithJitter = (numRetries: number) => {
  const baseBackoff = INITIAL_BACKOFF * 2 ** (numRetries - 1);
  const actualBackoff = Math.min(baseBackoff, MAX_BACKOFF);
  const jitter = actualBackoff * (Math.random() - 0.5);
  return actualBackoff + jitter;
};
