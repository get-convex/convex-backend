---
sidebar_label: "Indexes and Query Performance"
title: "Introduction to Indexes and Query Performance"
sidebar_position: 100
---

How do I ensure my Convex
[database queries](/docs/database/reading-data/reading-data.mdx) are fast and
efficient? When should I define an
[index](/docs/database/reading-data/indexes/indexes.md)? What is an index?

This document explains how you should think about query performance in Convex by
describing a simplified model of how queries and indexes function.

If you already have a strong understanding of database queries and indexes you
can jump straight to the reference documentation instead:

- [Reading Data](/docs/database/reading-data/reading-data.mdx)
- [Indexes](/docs/database/reading-data/indexes/indexes.md)

## A Library of Documents

You can imagine that Convex is a physical library storing documents as physical
books. In this world, every time you add a document to Convex with
[`db.insert("books", {...})`](/api/interfaces/server.GenericDatabaseWriter#insert)
a librarian places the book on a shelf.

By default, Convex organizes your documents in the order they were inserted. You
can imagine the librarian inserting documents left to right on a shelf.

If you run a query to find the first book like:

```ts
const firstBook = await ctx.db.query("books").first();
```

then the librarian could start at the left edge of the shelf and find the first
book. This is an extremely fast query because the librarian only has to look at
a single book to get the result.

Similarly, if we want to retrieve the last book that was inserted we could
instead do:

```ts
const lastBook = await ctx.db.query("books").order("desc").first();
```

This is the same query but we've swapped the order to descending. In the
library, this means that the librarian will start on the right edge of the shelf
and scan right-to-left. The librarian still only needs to look at a single book
to determine the result so this query is also extremely fast.

## Full Table Scans

Now imagine that someone shows up at the library and asks "what books do you
have by Jane Austen?"

This could be expressed as:

```ts
const books = await ctx.db
  .query("books")
  .filter((q) => q.eq(q.field("author"), "Jane Austen"))
  .collect();
```

This query is saying "look through all of the books, left-to-right, and collect
the ones where the `author` field is Jane Austen." To do this the librarian will
need to look through the entire shelf and check the author of every book.

This query is a _full table scan_ because it requires Convex to look at every
document in the table. The performance of this query is based on the number of
books in the library.

If your Convex table has a small number of documents, this is fine! Full table
scans should still be fast if there are a few hundred documents, but if the
table has many thousands of documents these queries will become slow.

In the library analogy, this kind of query is fine if the library has a single
shelf. As the library expands into a bookcase with many shelves or many
bookcases, this approach becomes infeasible.

## Card Catalogs

How can we more efficiently find books given an author?

One option is to re-sort the entire library by `author`. This will solve our
immediate problem but now our original queries for `firstBook` and `lastBook`
would become full table scans because we'd need to examine every book to see
which was inserted first/last.

Another option is to duplicate the entire library. We could purchase 2 copies of
every book and put them on 2 separate shelves: one shelf sorted by insertion
time and another sorted by author. This would work, but it's expensive. We now
need twice as much space for our library.

A better option is to build an _index_ on `author`. In the library, we could use
an old-school [card catalog](https://en.wikipedia.org/wiki/Library_catalog) to
organize the books by author. The idea here is that the librarian will write an
index card for each book that contains:

- The book's author
- The location of the book on the shelves

These index cards will be sorted by author and live in a separate organizer from
the shelves that hold the books. The card catalog should stay small because it
only has an index card per book (not the entire text of the book).

![Card Catalog](/img/card-catalog.jpg)

When a patron asks for "books by Jane Austen", the librarian can now:

1. Go to the card catalog and quickly find all of the cards for "Jane Austen".
2. For each card, go and find the book on the shelf.

This is quite fast because the librarian can quickly find the index cards for
Jane Austen. It's still a little bit of work to find the book for each card but
the number of index cards is small so this is quite fast.

## Indexes

Database indexes work based on the same concept! With Convex you can define an
_index_ with:

```ts noDialect title="convex/schema.ts"
import { defineSchema, defineTable } from "convex/server";
import { v } from "convex/values";

export default defineSchema({
  books: defineTable({
    author: v.string(),
    title: v.string(),
    text: v.string(),
  }).index("by_author", ["author"]),
});
```

then Convex will create a new index called `by_author` on `author`. This means
that your `books` table will now have an additional data structure that is
sorted by the `author` field.

You can query this index with:

```ts
const austenBooks = await ctx.db
  .query("books")
  .withIndex("by_author", (q) => q.eq("author", "Jane Austen"))
  .collect();
```

This query instructs Convex to go to the `by_author` index and find all the
entries where `doc.author === "Jane Austen"`. Because the index is sorted by
`author`, this is a very efficient operation. This means that Convex can execute
this query in the same manner that the librarian can:

1. Find the range of the index with entries for Jane Austen.
2. For each entry in that range, get the corresponding document.

The performance of this query is based on the number of documents where
`doc.author === "Jane Austen"` which should be quite small. We've dramatically
sped up the query!

## Backfilling and Maintaining Indexes

One interesting detail to think about is the work needed to create this new
structure. In the library, the librarian must go through every book on the shelf
and put a new index card for each one in the card catalog sorted by author. Only
after that can the librarian trust that the card catalog will give it correct
results.

The same is true for Convex indexes! When you define a new index, the first time
you run `npx convex deploy` Convex will need to loop through all of your
documents and index each one. This is why the first deploy after the creation of
a new index will be slightly slower than normal; Convex has to do a bit of work
for each document in your table.

Similarly, even after an index is defined, Convex will have to do a bit of extra
work to keep this index up to date as the data changes. Every time a document is
inserted, updated, or deleted in an indexed table, Convex will also update its
index entry. This is analogous to a librarian creating new index cards for new
books as they add them to the library.

If you are defining a few indexes there is no need to worry about the
maintenance cost. As you define more indexes, the cost to maintain them grows
because every `insert` needs to update every index. This is why Convex has a
limit of 32 indexes per table. In practice most applications define a handful of
indexes per table to make their important queries efficient.

## Indexing Multiple Fields

Now imagine that a patron shows up at the library and would like to check out
_Foundation_ by Isaac Asimov. Given our index on `author`, we can write a query
that uses the index to find all the books by Isaac Asimov and then examines the
title of each book to see if it's _Foundation_.

```ts
const foundation = await ctx.db
  .query("books")
  .withIndex("by_author", (q) => q.eq("author", "Isaac Asimov"))
  .filter((q) => q.eq(q.field("title"), "Foundation"))
  .unique();
```

This query describes how a librarian might execute the query. The librarian will
use the card catalog to find all of the index cards for Isaac Asimov's books.
The cards themselves don't have the title of the book so the librarian will need
to find every Asimov book on the shelves and look at its title to find the one
named _Foundation_. Lastly, this query ends with
[`.unique`](/api/interfaces/server.Query#unique) because we expect there to be
at most one result.

This query demonstrates the difference between filtering using
[`withIndex`](/api/interfaces/server.QueryInitializer#withIndex) and
[`filter`](/api/interfaces/server.Query#filter). `withIndex` only allows you to
restrict your query based on the index. You can only do operations that the
index can do efficiently like finding all documents with a given author.

`filter` on the other hand allows you to write arbitrary, complex expressions
but it won't be run using the index. Instead, `filter` expressions will be
evaluated on every document in the range.

Given all of this, we can conclude that **the performance of indexed queries is
based on how many documents are in the index range**. In this case, the
performance is based on the number of Isaac Asimov books because the librarian
will need to look at each one to examine its title.

Unfortunately, Isaac Asimov wrote
[a lot of books](<https://en.wikipedia.org/wiki/Isaac_Asimov_bibliography_(alphabetical)>).
Realistically even with 500+ books, this will be fast enough on Convex with the
existing index, but let's consider how we could improve it anyway.

One approach is to build a separate `by_title` index on `title`. This could let
us swap the work we do in `.filter` and `.withIndex` to instead be:

```ts
const foundation = await ctx.db
  .query("books")
  .withIndex("by_title", (q) => q.eq("title", "Foundation"))
  .filter((q) => q.eq(q.field("author"), "Isaac Asimov"))
  .unique();
```

In this query, we're efficiently using the index to find all the books called
_Foundation_ and then filtering through to find the one by Isaac Asimov.

This is okay, but we're still at risk of having a slow query because too many
books have a title of _Foundation_. An even better approach could be to build a
_compound_ index that indexes both `author` and `title`. Compound indexes are
indexes on an ordered list of fields.

```ts noDialect title="convex/schema.ts"
import { defineSchema, defineTable } from "convex/server";
import { v } from "convex/values";

export default defineSchema({
  books: defineTable({
    author: v.string(),
    title: v.string(),
    text: v.string(),
  }).index("by_author_title", ["author", "title"]),
});
```

In this index, books are sorted first by the author and then within each author
by title. This means that a librarian can use the index to jump to the Isaac
Asimov section and quickly find _Foundation_ within it.

Expressing this as a Convex query this looks like:

```ts
const foundation = await ctx.db
  .query("books")
  .withIndex("by_author_title", (q) =>
    q.eq("author", "Isaac Asimov").eq("title", "Foundation"),
  )
  .unique();
```

Here the index range expression tells Convex to only consider documents where
the author is Isaac Asimov and the title is _Foundation_. This is only a single
document so this query will be quite fast!

Because this index sorts by `author` and then by `title`, it also efficiently
supports queries like "All books by Isaac Asimov that start with F." We could
express this as:

```ts
const asimovBooksStartingWithF = await ctx.db
  .query("books")
  .withIndex("by_author_title", (q) =>
    q.eq("author", "Isaac Asimov").gte("title", "F").lt("title", "G"),
  )
  .collect();
```

This query uses the index to find books where
`author === "Isaac Asimov" && "F" <= title < "G"`. Once again, the performance
of this query is based on how many documents are in the index range. In this
case, that's just the Asimov books that begin with "F" which is quite small.

Also note that this index also supports our original query for "books by Jane
Austen." It's okay to only use the `author` field in an index range expression
and not restrict by title at all.

Lastly, imagine that a library patron asks for the book _The Three-Body Problem_
but they don't know the author's name. Our `by_author_title` index won't help us
here because it's sorted first by `author`, and then by `title`. The title, _The
Three-Body Problem_, could appear anywhere in the index!

The Convex TypeScript types in the `withIndex` make this clear because they
require that you compare index fields in order. Because the index is defined on
`["author", "title"]`, you must first compare the `author` with `.eq` before the
`title`.

In this case, the best option is probably to create the separate `by_title`
index to facilitate this query.

## Conclusions

Congrats! You now understand how queries and indexes work within Convex!

Here are the main points we've covered:

1. By default Convex queries are _full table scans_. This is appropriate for
   prototyping and querying small tables.
2. As your tables grow larger, you can improve your query performance by adding
   _indexes_. Indexes are separate data structures that order your documents for
   fast querying.
3. In Convex, queries use the _`withIndex`_ method to express the portion of the
   query that uses the index. The performance of a query is based on how many
   documents are in the index range expression.
4. Convex also supports _compound indexes_ that index multiple fields.

To learn more about queries and indexes, check out our reference documentation:

- [Reading Data](/docs/database/reading-data/reading-data.mdx)
- [Indexes](/docs/database/reading-data/indexes/indexes.md)
