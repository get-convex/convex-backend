import { MUTATION_TIMEOUT, QUERY_TIMEOUT } from "../types.js";
import { Scenario, nowSeconds, IScenario, Config } from "../scenario.js";
import { api } from "../convex/_generated/api.js";
import { ScenarioError } from "../metrics.js";
import { ConvexClient } from "convex/browser";
import { MessagesTable, rand } from "../convex/common.js";

/*
  Mutation inserts a row in the table and sends latency metrics on how long it
  takes for the mutation to complete and for the mutation to be observed by a query that is subscribed.
*/
export class ObserveInsert extends Scenario implements IScenario {
  table: MessagesTable;

  constructor(config: Config, withSearch: boolean) {
    const name = withSearch ? "ObserveInsertWithSearch" : "ObserveInsert";
    super(name, config);
    this.table = withSearch ? "messages_with_search" : "messages";
  }

  async sendMutation(client: ConvexClient, startTime: number, rand: number) {
    await client.mutation(api.insert.insertMessageWithArgs, {
      channel: "global",
      timestamp: startTime,
      rand,
      ballastCount: 0,
      count: 1,
      table: this.table,
    });
  }

  async run(client: ConvexClient) {
    const startTime = nowSeconds();
    const randomNumber = rand();
    const subscribe = this.subscribe(client, startTime, randomNumber);

    // Send the replace mutation or timeout
    await this.executeOrTimeoutWithLatency(
      this.sendMutation(client, startTime, randomNumber),
      MUTATION_TIMEOUT,
      "mutation_send_timeout",
      "mutation_completed",
      nowSeconds(),
    );

    // Wait until the mutation is observed or timeout
    await this.executeOrTimeoutWithLatency(
      subscribe!,
      QUERY_TIMEOUT,
      "mutation_observed_timeout",
      "mutation_observed",
      startTime,
    );
  }

  // Watch a query on the given offset of the table to observe the insert mutation
  async subscribe(
    client: ConvexClient,
    startTime: number,
    rand: number,
  ): Promise<void> {
    await this.waitForQuery(
      client,
      api.query_index.queryMessagesWithArgs,
      {
        channel: "global",
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
