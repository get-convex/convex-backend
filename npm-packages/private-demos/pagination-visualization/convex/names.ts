import { mutation, query } from "./_generated/server";
import { paginationOptsValidator } from "convex/server";
import { v } from "convex/values";

export const getPeople = query({
  args: {
    paginationOpts: paginationOptsValidator,
  },
  handler: async ({ db }, { paginationOpts }) => {
    return await db
      .query("people")
      .withIndex("by_name")
      .paginate(paginationOpts);
  },
});

export const seed = mutation({
  args: {
    names: v.array(v.string()),
  },
  handler: async ({ db }, { names }) => {
    // Remove all existing rows
    for await (const row of db.query("people")) {
      await db.delete(row._id);
    }

    for (const name of names) {
      await db.insert("people", {
        name,
      });
    }
  },
});

export const addPerson = mutation({
  args: {
    name: v.string(),
  },
  handler: async ({ db }, { name }) => {
    await db.insert("people", {
      name,
    });
  },
});

export const deletePerson = mutation({
  args: {
    id: v.id("people"),
  },
  handler: async ({ db }, { id }) => {
    await db.delete(id);
  },
});
