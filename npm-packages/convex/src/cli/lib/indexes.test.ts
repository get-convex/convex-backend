import { describe, test, expect } from "vitest";
import {
  IndexMetadata,
  toDeveloperIndexConfig,
  formatIndex,
} from "./indexes.js";
import { DeveloperIndexConfig } from "./deployApi/finishPush.js";
import chalk from "chalk";

describe("toDeveloperIndexConfig", () => {
  test("converts database IndexMetadata to DeveloperIndexConfig", () => {
    const databaseIndex: IndexMetadata = {
      table: "messages",
      name: "by_user_and_timestamp",
      fields: ["userId", "timestamp"],
      backfill: {
        state: "done",
      },
      staged: false,
    };

    const result = toDeveloperIndexConfig(databaseIndex);

    expect(result).toEqual({
      type: "database",
      name: "messages.by_user_and_timestamp",
      fields: ["userId", "timestamp"],
      staged: false,
    });
  });

  test("converts text search IndexMetadata to DeveloperIndexConfig", () => {
    const searchIndex: IndexMetadata = {
      table: "articles",
      name: "search_by_content",
      fields: {
        searchField: "content",
        filterFields: ["category", "status"],
      },
      backfill: {
        state: "done",
      },
      staged: false,
    };

    const result = toDeveloperIndexConfig(searchIndex);

    expect(result).toEqual({
      type: "search",
      name: "articles.search_by_content",
      searchField: "content",
      filterFields: ["category", "status"],
      staged: false,
    });
  });

  test("converts vector search IndexMetadata to DeveloperIndexConfig", () => {
    const vectorIndex: IndexMetadata = {
      table: "documents",
      name: "embedding_index",
      fields: {
        dimensions: 1536,
        vectorField: "embedding",
        filterFields: ["userId", "type"],
      },
      backfill: {
        state: "done",
      },
      staged: true,
    };

    const result = toDeveloperIndexConfig(vectorIndex);

    expect(result).toEqual({
      type: "vector",
      name: "documents.embedding_index",
      dimensions: 1536,
      vectorField: "embedding",
      filterFields: ["userId", "type"],
      staged: true,
    });
  });
});

describe("formatIndex", () => {
  test("formats database index with multiple fields", () => {
    const databaseIndex: DeveloperIndexConfig = {
      type: "database",
      name: "messages.by_user_and_timestamp",
      fields: ["userId", "timestamp"],
    };

    const result = formatIndex(databaseIndex);

    const expected = `messages.${chalk.bold("by_user_and_timestamp")}   ${chalk.gray(`${chalk.underline("userId")}, ${chalk.underline("timestamp")}`)}`;
    expect(result).toEqual(expected);
  });

  test("formats text index without filter fields", () => {
    const searchIndex: DeveloperIndexConfig = {
      type: "search",
      name: "articles.search_by_content",
      searchField: "content",
      filterFields: [],
    };

    const result = formatIndex(searchIndex);

    const expected = `articles.${chalk.bold("search_by_content")} ${chalk.gray(`${chalk.cyan("(text)")}   ${chalk.underline("content")}`)}`;
    expect(result).toEqual(expected);
  });

  test("formats text index with filter fields", () => {
    const searchIndex: DeveloperIndexConfig = {
      type: "search",
      name: "articles.search_by_content",
      searchField: "content",
      filterFields: ["category", "status"],
    };

    const result = formatIndex(searchIndex);

    const expected = `articles.${chalk.bold("search_by_content")} ${chalk.gray(`${chalk.cyan("(text)")}   ${chalk.underline("content")}, filters on ${chalk.underline("category")}, ${chalk.underline("status")}`)}`;
    expect(result).toEqual(expected);
  });

  test("formats vector index with single filter field", () => {
    const vectorIndex: DeveloperIndexConfig = {
      type: "vector",
      name: "documents.embedding_index",
      dimensions: 1536,
      vectorField: "embedding",
      filterFields: ["userId"],
    };

    const result = formatIndex(vectorIndex);

    const expected = `documents.${chalk.bold("embedding_index")} ${chalk.gray(`${chalk.cyan("(vector)")}   ${chalk.underline("embedding")} (1536 dimensions), filter on ${chalk.underline("userId")}`)}`;
    expect(result).toEqual(expected);
  });

  test("formats vector index with multiple filter fields", () => {
    const vectorIndex: DeveloperIndexConfig = {
      type: "vector",
      name: "documents.embedding_index",
      dimensions: 768,
      vectorField: "embedding",
      filterFields: ["userId", "type", "category"],
    };

    const result = formatIndex(vectorIndex);

    const expected = `documents.${chalk.bold("embedding_index")} ${chalk.gray(`${chalk.cyan("(vector)")}   ${chalk.underline("embedding")} (768 dimensions), filters on ${chalk.underline("userId")}, ${chalk.underline("type")}, ${chalk.underline("category")}`)}`;
    expect(result).toEqual(expected);
  });
});
