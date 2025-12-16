import { describe, test, expect } from "vitest";
import { DeveloperIndexConfig } from "./deployApi/finishPush.js";
import { chalkStderr } from "chalk";
import { formatIndex } from "./indexes.js";

describe("formatIndex", () => {
  test("formats database index with multiple fields", () => {
    const databaseIndex: DeveloperIndexConfig = {
      type: "database",
      name: "messages.by_user_and_timestamp",
      fields: ["userId", "timestamp"],
    };

    const result = formatIndex(databaseIndex);

    const expected = `messages.${chalkStderr.bold("by_user_and_timestamp")}   ${chalkStderr.gray(`${chalkStderr.underline("userId")}, ${chalkStderr.underline("timestamp")}`)}`;
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

    const expected = `articles.${chalkStderr.bold("search_by_content")} ${chalkStderr.gray(`${chalkStderr.cyan("(text)")}   ${chalkStderr.underline("content")}`)}`;
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

    const expected = `articles.${chalkStderr.bold("search_by_content")} ${chalkStderr.gray(`${chalkStderr.cyan("(text)")}   ${chalkStderr.underline("content")}, filters on ${chalkStderr.underline("category")}, ${chalkStderr.underline("status")}`)}`;
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

    const expected = `documents.${chalkStderr.bold("embedding_index")} ${chalkStderr.gray(`${chalkStderr.cyan("(vector)")}   ${chalkStderr.underline("embedding")} (1536 dimensions), filter on ${chalkStderr.underline("userId")}`)}`;
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

    const expected = `documents.${chalkStderr.bold("embedding_index")} ${chalkStderr.gray(`${chalkStderr.cyan("(vector)")}   ${chalkStderr.underline("embedding")} (768 dimensions), filters on ${chalkStderr.underline("userId")}, ${chalkStderr.underline("type")}, ${chalkStderr.underline("category")}`)}`;
    expect(result).toEqual(expected);
  });
});
