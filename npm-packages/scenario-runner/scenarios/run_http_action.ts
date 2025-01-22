import { HTTP_ACTION_TIMEOUT } from "../types.js";
import { Scenario, nowSeconds, IScenario, Config } from "../scenario.js";
import { ScenarioError } from "../metrics.js";
import { ConvexClient } from "convex/browser";
import { RoutableMethod } from "convex/server";
import { api } from "../convex/_generated/api.js";

export class RunHttpAction extends Scenario implements IScenario {
  path: string;
  method: RoutableMethod;

  constructor(config: Config, path: string, method: RoutableMethod) {
    super("RunHttpAction", config);
    this.path = path;
    this.method = method;
  }

  async run(client: ConvexClient) {
    const siteUrl = await client.query(api.http.siteUrl, {});

    await this.executeOrTimeoutWithLatency(
      fetch(new URL(this.path, siteUrl), {
        method: this.method,
      })
        .then((res) => {
          if (!res.ok) {
            this.sendDefaultError(
              new Error(`HTTP action returned with status ${res.status}`),
            );
          }
        })
        .catch((reason) => {
          this.sendDefaultError(new Error(`HTTP action failed ${reason}`));
        }),
      HTTP_ACTION_TIMEOUT,
      "http_action_timeout",
      "http_action",
      nowSeconds(),
      this.path,
    );
  }

  defaultErrorName(): ScenarioError {
    return "http_action";
  }
}
