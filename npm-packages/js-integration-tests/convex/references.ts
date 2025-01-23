import { Id } from "./_generated/dataModel";
import { mutation, query } from "./_generated/server";

export const createGraph = mutation(async ({ db }) => {
  const refs = new Map();
  for (const name of "abcde") {
    const docId = await db.insert("nodes", { name });
    refs.set(name, docId);
  }
  const edges = [
    ["a", "b"],
    ["b", "c"],
    ["c", "d"],
    ["d", "a"],
    ["e", "a"],
    ["e", "b"],
    ["e", "c"],
  ];
  for (const [srcName, dstName] of edges) {
    const src = refs.get(srcName);
    const dst = refs.get(dstName);
    await db.insert("edges", { src, dst });
  }
});

export const incomingEdges = query(
  async ({ db }, { name }: { name: string }) => {
    const node = await db
      .query("nodes")
      .filter((q) => q.eq(q.field("name"), name))
      .unique();

    const out = [];
    const query = db
      .query("edges")
      .filter((q) => q.eq(q.field("dst"), node!._id));
    for await (const row of query) {
      const node = (await db.get(row.src))!;
      out.push(node.name);
    }
    return out;
  },
);

export const addNode = mutation(async ({ db }, { name }: { name: string }) => {
  return await db.insert("nodes", { name });
});

export const addEdge = mutation(
  async ({ db }, { src, dst }: { src: Id<"nodes">; dst: Id<"nodes"> }) => {
    await db.insert("edges", { src, dst });
  },
);

export const deleteGraph = mutation(async ({ db }) => {
  for await (const { _id } of db.query("edges")) {
    await db.delete(_id);
  }
  for await (const { _id } of db.query("nodes")) {
    await db.delete(_id);
  }
});
