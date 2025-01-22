import { TableDefinition, defineTable } from "convex/server";
import { v } from "convex/values";

function table({
  searchIndex = false,
  vectorIndex = false,
}: {
  searchIndex?: boolean;
  vectorIndex?: boolean;
}) {
  let t = defineTable({
    author: v.string(),
    body: v.string(),
    channel: v.string(),
    embedding: v.array(v.float64()),
  });
  console.log(t);
  if (searchIndex) {
    t = t.searchIndex("search_body", {
      searchField: "body",
      filterFields: ["channel"],
    });
  }
  if (vectorIndex) {
    t = t.vectorIndex("by_embedding", {
      vectorField: "embedding",
      dimensions: 2,
      filterFields: ["channel"],
    });
  }
  console.log("returning", t);
  return t;
}

export function tables(count: {
  normal: number;
  withSearchIndex: number;
  withVectorIndex: number;
}) {
  const tables: Record<string, TableDefinition> = {};
  for (let i = 0; i < count.normal; i++) {
    const name = `table_${i}`;
    tables[name] = table({ searchIndex: false, vectorIndex: false });
  }
  for (let i = 0; i < count.withSearchIndex; i++) {
    const name = `table_search_${i}`;
    tables[name] = table({ searchIndex: true, vectorIndex: false });
  }
  for (let i = 0; i < count.withVectorIndex; i++) {
    const name = `table_vector_${i}`;
    tables[name] = table({ searchIndex: false, vectorIndex: true });
  }
  return tables;
}
