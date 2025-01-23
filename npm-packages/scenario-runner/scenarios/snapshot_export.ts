import { Config, Scenario } from "../scenario";
import { EXPORT_TIMEOUT } from "../types";
import { makeFunctionReference } from "convex/server";
import { ScenarioError } from "../metrics";
import { ConvexClient } from "convex/browser";
import { Readable } from "stream";

async function streamPrefix(stream: Readable) {
  for await (const chunk of stream) {
    return chunk;
  }
  return "";
}

type Headers = {
  Authorization: string;
};

export class SnapshotExport extends Scenario implements Scenario {
  adminKey: string;
  headers: Headers;
  constructor(config: Config, adminKey: string) {
    super("SnapshotExport", config);
    this.adminKey = adminKey;
    this.headers = {
      Authorization: `Convex ${adminKey}`,
    };
  }

  async run(client: ConvexClient) {
    client.setAdminAuth(this.adminKey);
    // Request export
    const res = await fetch(`${this.deploymentUrl}/api/export/request/zip`, {
      method: "POST",
      headers: this.headers,
    });
    if (res.status === 200) {
      this.sendCountMetric(1, "request_export_succeeded");
    } else {
      const data = await res.text();
      this.sendError(data, "request_export_failed");
    }

    try {
      await this.executeOrTimeout(
        this.subscribe(client),
        EXPORT_TIMEOUT,
        "export_timeout",
      );
    } catch (err: any) {
      this.sendError(err, "snapshot_export_failed");
    }
  }

  async subscribe(client: ConvexClient) {
    try {
      await this.subscribeInner(client);
    } catch (err: any) {
      this.sendError(err, "snapshot_export_failed");
    }
  }

  async subscribeInner(client: ConvexClient) {
    // Watch for export completion
    const result = await this.waitForQuery(
      client,
      makeFunctionReference<"query">("_system/frontend/latestExport"),
      {},
      (result) => {
        if (result.state === "in_progress") {
          this.sendCountMetric(1, "export_in_progress");
        } else if (result.state === "completed") {
          return true;
        }
        return false;
      },
    );
    this.sendCountMetric(1, "export_completed");
    const snapshotTs = result.start_ts.toString();
    const res = await fetch(
      `${this.deploymentUrl}/api/export/zip/${snapshotTs}`,
      { headers: this.headers },
    );
    const body = Readable.fromWeb(res.body! as any);
    if (res.status === 200) {
      this.sendCountMetric(1, "get_export_succeeded");
      // Consume the body to make sure it's not buffered anywhere.
      for await (const _ of body) {
        // Do nothing
      }
    } else {
      this.sendError(await streamPrefix(body), "get_export_failed");
    }
  }

  defaultErrorName(): ScenarioError {
    return "snapshot_failure";
  }
}
