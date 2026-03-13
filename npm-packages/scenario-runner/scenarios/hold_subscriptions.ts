import { MUTATION_TIMEOUT } from "../types.js";
import { Scenario, nowSeconds, IScenario, Config } from "../scenario.js";
import { api } from "../convex/_generated/api.js";
import { ScenarioError } from "../metrics.js";
import { ConvexClient } from "convex/browser";
import { rand } from "../convex/common.js";

/**
 * Hold many subscriptions open for an extended duration, optionally
 * invalidating a subset at regular intervals.
 */
export class HoldSubscriptions extends Scenario implements IScenario {
  numSubscriptions: number;
  holdDurationSecs: number;
  invalidationIntervalSecs: number | undefined;
  numInvalidations: number | undefined;

  constructor(
    config: Config,
    numSubscriptions: number,
    holdDurationSecs: number,
    invalidationIntervalSecs?: number,
    numInvalidations?: number,
  ) {
    const name = "HoldSubscriptions";
    super(name as any, config);
    this.numSubscriptions = numSubscriptions;
    this.holdDurationSecs = holdDurationSecs;
    this.invalidationIntervalSecs = invalidationIntervalSecs;
    this.numInvalidations = numInvalidations;
  }

  async run(client: ConvexClient) {
    // Create subscriptions with unique channels so we can target invalidations.
    const channels: string[] = [];
    for (let i = 0; i < this.numSubscriptions; i++) {
      const channel = `hold-sub-${i}-${rand()}`;
      channels.push(channel);
      // Fire-and-forget subscription: isReady always returns false so the
      // promise never resolves, but the subscription stays open.
      void this.waitForQuery(
        client,
        api.query_index.queryMessagesWithArgs,
        {
          channel,
          rand: rand(),
          limit: 10,
          table: "messages",
        },
        () => false,
      );
    }

    if (
      this.invalidationIntervalSecs === undefined ||
      this.numInvalidations === undefined
    ) {
      // Static mode: just hold subscriptions for the duration.
      await new Promise((r) => setTimeout(r, this.holdDurationSecs * 1000));
    } else {
      // Invalidation mode: periodically mutate a subset of subscribed channels.
      const intervalMs = this.invalidationIntervalSecs * 1000;
      const endTime = Date.now() + this.holdDurationSecs * 1000;

      while (Date.now() + intervalMs < endTime) {
        await new Promise((r) => setTimeout(r, intervalMs));

        // Pick numInvalidations random channels to invalidate.
        for (let j = 0; j < this.numInvalidations; j++) {
          const idx = Math.floor(Math.random() * channels.length);
          const channel = channels[idx];
          const t0 = nowSeconds();
          const mutation = client.mutation(api.insert.insertMessageWithArgs, {
            channel,
            timestamp: t0,
            rand: rand(),
            ballastCount: 0,
            count: 1,
            table: "messages",
          });
          await this.executeOrTimeoutWithLatency(
            mutation.then(() => {}),
            MUTATION_TIMEOUT * 50,
            "invalidation_timeout",
            "invalidation_completed",
            t0,
          );
        }
      }

      // Wait out any remaining time.
      const remaining = endTime - Date.now();
      if (remaining > 0) {
        await new Promise((r) => setTimeout(r, remaining));
      }
    }
  }

  defaultErrorName(): ScenarioError {
    return "mutation";
  }
}
