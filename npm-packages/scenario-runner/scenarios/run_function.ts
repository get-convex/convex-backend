import {
  ACTION_TIMEOUT,
  FnType,
  MUTATION_TIMEOUT,
  QUERY_TIMEOUT,
} from "../types.js";
import { Scenario, nowSeconds, IScenario, Config } from "../scenario.js";
import { ScenarioError } from "../metrics.js";
import { ConvexClient } from "convex/browser";
import { makeFunctionReference } from "convex/server";
import { rand } from "../convex/common.js";

export class RunFunction extends Scenario implements IScenario {
  path: string;
  fnType: "query" | "mutation" | "action";

  constructor(config: Config, path: string, fnType: FnType) {
    super("RunFunction", config);
    this.path = path;
    this.fnType = fnType;
  }

  async run(client: ConvexClient) {
    switch (this.fnType) {
      case "query": {
        const fn = makeFunctionReference<"query">(this.path);
        await this.executeOrTimeoutWithLatency(
          client.query(fn, { cacheBreaker: rand() }),
          QUERY_TIMEOUT,
          "query_timeout",
          this.fnType,
          nowSeconds(),
          this.path,
        );
        break;
      }
      case "mutation": {
        const fn = makeFunctionReference<"mutation">(this.path);
        await this.executeOrTimeoutWithLatency(
          client.mutation(fn, {}),
          MUTATION_TIMEOUT,
          "mutation_timeout",
          this.fnType,
          nowSeconds(),
          this.path,
        );
        break;
      }
      case "action": {
        const fn = makeFunctionReference<"action">(this.path);
        await this.executeOrTimeoutWithLatency(
          client.action(fn, {}),
          ACTION_TIMEOUT,
          "action_timeout",
          this.fnType,
          nowSeconds(),
          this.path,
        );
        break;
      }
      default:
        throw new Error(`Unknown function type: ${this.fnType}`);
    }
  }

  defaultErrorName(): ScenarioError {
    return this.fnType;
  }
}
