import { MUTATION_TIMEOUT, QUERY_TIMEOUT } from "../types.js";
import { Scenario, nowSeconds, IScenario, Config } from "../scenario.js";
import { api } from "../convex/_generated/api.js";
import { ScenarioError } from "../metrics.js";
import { ConvexClient } from "convex/browser";
import { MessagesTable, rand } from "../convex/common.js";
import { writeFileSync } from "node:fs";

/**
 * Subscribe to many queries and invalidate a small number of them.
 */
export class ManyIntersections extends Scenario implements IScenario {
  table: MessagesTable;
  numSubscriptions: number;
  numDocumentsPerMutation: number;

  constructor(config: Config, numSubscriptions = 1000) {
    const name = "ManyIntersections";
    super(name as any, config);
    this.table = "messages";
    this.numSubscriptions = numSubscriptions;
    this.numDocumentsPerMutation = 100;
  }

  async sendMutation(client: ConvexClient, startTime: number, rand: number) {
    await client.mutation(api.insert.insertMessagesWithArgs, {
      channel: "global",
      timestamp: startTime,
      rand,
      ballastCount: 0,
      count: 1,
      table: this.table,
      n: this.numDocumentsPerMutation,
    });
  }

  async run(client: ConvexClient) {
    writeFileSync(
      "/Users/tomb/log.txt",
      `Making ${this.numDocumentsPerMutation} document writes after ${this.numSubscriptions} subs at ${new Date().toTimeString().split(" ")[0]}...`,
      {
        encoding: "utf-8",
      },
    );
    const startTime = nowSeconds();
    const _ = range(this.numSubscriptions).map(() => {
      void this.subscribe(client, startTime, rand(), `channel-${rand()}`);
    });

    // Wait a sec
    await new Promise((r) => setTimeout(r, 1000));

    const t0 = nowSeconds();
    const randomNumber = rand();
    const subscribe = this.subscribe(client, startTime, randomNumber);

    // Send the replace mutation or timeout
    await this.executeOrTimeoutWithLatency(
      this.sendMutation(client, startTime, randomNumber),
      MUTATION_TIMEOUT * 50,
      "mutation_send_timeout",
      "mutation_completed",
      t0,
    );

    // Wait until the mutation is observed or timeout
    await this.executeOrTimeoutWithLatency(
      subscribe!,
      QUERY_TIMEOUT * 50,
      "mutation_observed_timeout",
      "mutation_observed",
      t0,
    );
  }

  // Watch a query on the given offset of the table to observe the insert mutation
  async subscribe(
    client: ConvexClient,
    startTime: number,
    rand: number,
    channel = "global",
  ): Promise<void> {
    await this.waitForQuery(
      client,
      api.query_index.queryMessagesWithArgs,
      {
        channel,
        rand,
        limit: 10,
        table: this.table,
      },
      (result) => {
        for (const doc of result || []) {
          if (doc.rand === rand && doc.timestamp! >= startTime) {
            return true;
          }
        }
        return false;
      },
    );
  }

  defaultErrorName(): ScenarioError {
    return "mutation";
  }
}

function range(n: number): number[] {
  return Array.from({ length: n }, (_, i) => i);
}
