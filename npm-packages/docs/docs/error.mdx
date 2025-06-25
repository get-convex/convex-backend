---
description: "Understand specific errors thrown by Convex"
---

# Errors and Warnings

This page explains specific errors thrown by Convex.

See [Error Handling](/functions/error-handling/error-handling.mdx) to learn
about handling errors in general.

<div id="occ-failure"></div>

## Write conflict: Optimistic concurrency control \{#1}

This system error is thrown when a mutation repeatedly fails due to conflicting
changes from parallel mutation executions.

### Example A

A mutation `updateCounter` always updates the same document:

```ts
export const updateCounter = mutation({
  args: {},
  handler: async (ctx) => {
    const doc = await ctx.db.get(process.env.COUNTER_ID);
    await ctx.db.patch(doc._id, { value: doc.value + 1 });
  },
});
```

If this mutation is called many times per second, many of its executions will
conflict with each other. Convex internally does several retries to mitigate
this concern, but if the mutation is called more rapidly than Convex can execute
it, some of the invocations will eventually throw this error:

<ErrorExample name="updateCounter">
  Documents read from or written to the table "counters" changed while this
  mutation was being run and on every subsequent retry. Another call to this
  mutation changed the document with ID "123456789101112".
</ErrorExample>

The error message will note the table name, which mutation caused the conflict
(in this example its another call to the same mutation), and one document ID
which was part of the conflicting change.

### Example B

Mutation `writeCount` depends on the entire `tasks` table:

```ts
export const writeCount = mutation({
  args: {
    target: v.id("counts"),
  },
  handler: async (ctx, args) => {
    const tasks = await ctx.db.query("tasks").collect();
    await ctx.db.patch(args.target, { value: tasks });
  },
});

export const addTask = mutation({
  args: {
    text: v.string(),
  },
  handler: async (ctx, args) => {
    await ctx.db.insert("tasks", { text: args.text });
  },
});
```

If the mutation `writeCount` is called at the same time as many calls to
`addTask` are made, either of the mutations can fail with this error. This is
because any change to the `"tasks"` table will conflict with the `writeCount`
mutation:

<ErrorExample name="writeCount">
  Documents read from or written to the table "tasks" changed while this
  mutation was being run and on every subsequent retry. A call to "addTask"
  changed the document with ID "123456789101112".
</ErrorExample>

### Remediation

To fix this issue:

1. Make sure that your mutations only read the data they need. Consider reducing
   the amount of data read by using indexed queries with
   [selective index range expressions](https://docs.convex.dev/database/indexes/).
2. Make sure you are not calling a mutation an unexpected number of times,
   perhaps from an action inside a loop.
3. Design your data model such that it doesn't require making many writes to the
   same document.

### Resources

- Learn more about [optimistic concurrency control](/database/advanced/occ.md).
- See this [Stack post](https://stack.convex.dev/waitlist) for an example of
  designing an app to avoid mutation conflicts.
