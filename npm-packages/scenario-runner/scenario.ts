import { Event, Metric } from "./types.js";
import {
  ScenarioName as ScenarioName,
  ScenarioLatencyMetric,
  ScenarioError,
  ScenarioCountMetric,
} from "./metrics.js";
import { WebSocket } from "ws";
import { ConvexClient } from "convex/browser";
import {
  FunctionArgs,
  FunctionReference,
  FunctionReturnType,
} from "convex/server";

export type Config = {
  loadGenWS: WebSocket;
  deploymentUrl: string;
  provisionerInfo: ProvisionerInfo | undefined;
};

export type ProvisionerInfo = {
  provisionHost: string;
  accessToken: string;
  deploymentName: string;
  deploymentId: number;
};

export function nowSeconds(): number {
  return performance.now() / 1000;
}
export abstract class Scenario {
  private cleanUpFunctions: (() => void)[];
  loadGenWS: WebSocket;
  deploymentUrl: string;
  provisionerInfo: ProvisionerInfo | undefined;

  constructor(
    public name: ScenarioName,
    config: Config,
  ) {
    this.cleanUpFunctions = [];
    this.loadGenWS = config.loadGenWS;
    this.deploymentUrl = config.deploymentUrl;
    this.provisionerInfo = config.provisionerInfo;
  }

  abstract run(client: ConvexClient): Promise<void>;

  abstract defaultErrorName(): ScenarioError;

  cleanUp() {
    for (const cleanUpFunction of this.cleanUpFunctions) {
      cleanUpFunction();
    }
    this.cleanUpFunctions = [];
  }

  /**
   * Subscribe to a query and return a promise for the first result
   * received for which isReady(result) returns true.
   *
   * The subscription will remain active until the scenario ends but
   */
  waitForQuery<Query extends FunctionReference<"query", "public">>(
    client: ConvexClient,
    query: Query,
    args: FunctionArgs<Query>,
    isReady: (result: FunctionReturnType<Query>) => boolean,
  ): Promise<FunctionReturnType<Query>> {
    return new Promise((resolve, reject) => {
      const unsubscribe = client.onUpdate(
        query,
        args,
        (result) => {
          if (isReady(result)) {
            resolve(result);
          }
        },
        reject,
      );
      this.registerCleanUp(unsubscribe);
    });
  }

  /**
   * Register clean up work to be performed after this scenario runs.
   *
   * This is guaranteed to run even if the scenario times out or hits another
   * error. Use this for things like unsubscribe callbacks.
   */
  registerCleanUp(cleanUpFunction: () => void) {
    this.cleanUpFunctions.push(cleanUpFunction);
  }

  sendCountMetric(value: number, name: ScenarioCountMetric, path?: string) {
    const metric: Metric = {
      type: "Count",
      value,
      name,
      scenario: this.name,
      path,
    };
    this.sendEvent(metric);
  }

  sendLatencyMetric(value: number, name: ScenarioLatencyMetric, path?: string) {
    const metric: Metric = {
      type: "Latency",
      value,
      name,
      scenario: this.name,
      path,
    };
    this.sendEvent(metric);
  }

  sendDefaultError(err: any) {
    this.sendError(err, this.defaultErrorName());
  }

  sendError(err: any, name: ScenarioError) {
    console.error(`Error in scenario ${this.name}: ${name}`, err);
    const error = {
      msg: err.toString(),
      name,
      scenario: this.name,
    };
    this.sendEvent(error);
  }

  private sendEvent(event: Event) {
    const msg = JSON.stringify(event);
    this.loadGenWS.send(msg);
  }
  async executeOrTimeout(
    promise: Promise<void>,
    timeoutDuration: number,
    timeoutMetricName: ScenarioCountMetric,
    path?: string,
  ) {
    const result = await Promise.race([
      promise,
      new Promise<string>((resolve) =>
        setTimeout(() => {
          resolve("timeout");
        }, timeoutDuration),
      ),
    ]);
    if (result === "timeout") {
      this.sendCountMetric(1, timeoutMetricName, path);
    }
  }

  async executeOrTimeoutWithLatency(
    promise: Promise<void>,
    timeoutDuration: number,
    timeoutMetricName: ScenarioCountMetric,
    latencyMetricName: ScenarioLatencyMetric,
    t0: number,
    path?: string,
  ) {
    const promiseWithLatency = async () => {
      await promise;
      this.sendLatencyMetric(nowSeconds() - t0, latencyMetricName, path);
    };
    await this.executeOrTimeout(
      promiseWithLatency(),
      timeoutDuration,
      timeoutMetricName,
      path,
    );
  }
}

export interface IScenario {
  /// Name of scenario
  name: ScenarioName;
  /// URL of the backend instance ScenarioRunner runs queries and mutations against
  deploymentUrl: string;
  /// Websocket connected to Rust LoadGenerator
  loadGenWS: WebSocket;
  /// Send count metrics to Rust LoadGenerator via websocket
  sendCountMetric: (value: number, name: ScenarioCountMetric) => void;
  /// Send latency metrics to Rust LoadGenerator via websocket
  sendLatencyMetric: (value: number, name: ScenarioLatencyMetric) => void;
  /// Send errors to Rust LoadGenerator via websocket
  sendError: (err: any, name: ScenarioError) => void;
  /// Run the scenario
  run: (client: ConvexClient) => void;
}
