import { Config, IScenario, nowSeconds, Scenario } from "../scenario";
import { api } from "../convex/_generated/api";
import { QUERY_TIMEOUT } from "../types";
import { ScenarioError } from "../metrics";
import { ConvexClient } from "convex/browser";
import { rand } from "../convex/common";

export class Search extends Scenario implements IScenario {
  constructor(config: Config) {
    super("Search", config);
  }

  async runInner(client: ConvexClient) {
    // Find a random document in the existing table.
    const randomNumber = rand();
    const indexResults = await client.query(
      api.query_index.queryMessagesWithArgs,
      {
        channel: "global",
        rand: randomNumber,
        limit: 1,
        table: "messages_with_search",
      },
    );
    if (indexResults.length === 0) {
      // We may have no documents in "global".
      return;
    }
    const document = indexResults[0];

    // Try querying for the document directly.
    const t0 = nowSeconds();
    const searchResults = await client.query(api.search.default, {
      channel: document.channel,
      body: `nonexistentWord ${document.body} anotherNonexistentWord`,
      limit: 10,
    });
    this.sendLatencyMetric(nowSeconds() - t0, "search");

    // Ignore cases where we don't find any documents. This could happen
    // due to concurrrent updates or deletes.
    if (searchResults.length === 0) {
      return;
    }
    const searchDocument = searchResults[0];

    // We're not guaranteed it's the exact same document, but at least
    // sanity check that its channel and body match.
    const channelMatches = searchDocument.channel === document.channel;
    const bodyMatches = searchDocument.body.indexOf(document.body) !== -1;
    if (!(channelMatches && bodyMatches)) {
      const docStillExists =
        (await client.query(api.query_index.queryMessagesById, {
          id: document._id,
        })) !== null;
      const docInResults =
        searchResults.find(
          (doc) =>
            doc.channel === document.channel &&
            doc.body.indexOf(document.body) !== -1,
        ) !== undefined;

      const areAllResultsExactMatches = searchResults.find(
        (doc) => doc.channel !== document.channel || doc.body !== document.body,
      );

      if (docStillExists) {
        this.sendError(
          `Document ${JSON.stringify(
            searchDocument,
          )} doesn't match 0th result${JSON.stringify(
            document,
          )}. Is in results: ${docInResults}. Are all results exact matches: ${areAllResultsExactMatches}. Other results: ${searchResults
            .slice(1)
            .map((doc) => JSON.stringify(doc))}`,
          "search_document_mismatch",
        );
      }
    }
  }

  async run(client: ConvexClient) {
    const result = this.runInner(client);
    // We execute two queries in the search scenario.
    await this.executeOrTimeout(result, QUERY_TIMEOUT * 2, "search_timeout");
  }

  defaultErrorName(): ScenarioError {
    return "search";
  }
}
