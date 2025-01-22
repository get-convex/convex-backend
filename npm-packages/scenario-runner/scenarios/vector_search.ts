import { ConvexClient } from "convex/browser";
import { Config, IScenario, nowSeconds, Scenario } from "../scenario";
import { api } from "../convex/_generated/api";
import { QUERY_TIMEOUT } from "../types";
import { ScenarioError } from "../metrics";

export class VectorSearch extends Scenario implements IScenario {
  constructor(config: Config) {
    super("VectorSearch", config);
  }

  async runInner(client: ConvexClient) {
    // Find a random document in the existing table.
    const document = await client.query(api.openclaurd.findRandomOpenclaurd, {
      cacheBreaker: Math.random(),
    });
    if (!document) {
      return;
    }

    // Try searching for the document with vector search.
    const t0 = nowSeconds();
    const result = await client.action(api.vectorSearch.default, {
      vector: document.embedding,
      users: [document.user],
      limit: 1,
    });
    this.sendLatencyMetric(nowSeconds() - t0, "vector_search");

    // Skip if we have no results - this could happen due to concurrrent
    // updates or deletes.
    if (result.length === 0) {
      return;
    }
    const searchDocument = result[0];

    // We're not guaranteed it's the exact same document, but at least
    // sanity check that its channel and body match.
    const idMatches = searchDocument._id === document._id;
    const scoreIsOne = searchDocument._score > 0.99;
    if (!(idMatches && scoreIsOne)) {
      const byIdDoc = await client.query(api.openclaurd.queryOpenclaurdById, {
        id: document._id,
      });
      this.sendError(
        `Document ${JSON.stringify(searchDocument)} doesn't match doc ${
          document._id
        }, or or score is unexpected ${
          searchDocument._score
        }, original document is ${byIdDoc !== null ? "present" : "missing"},
        user is ${document.user}, rand is ${document.rand}`,
        "vector_search_document_mismatch",
      );
    }
  }

  async run(client: ConvexClient) {
    const result = this.runInner(client);
    // We execute two queries in the search scenario.
    await this.executeOrTimeout(
      result,
      QUERY_TIMEOUT * 2,
      "vector_search_timeout",
    );
  }

  defaultErrorName(): ScenarioError {
    return "vector_search";
  }
}
