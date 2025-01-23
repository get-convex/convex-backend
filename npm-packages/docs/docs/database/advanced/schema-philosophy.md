---
title: Schema Philosophy
sidebar_position: 450
---

With Convex there is no need to write any `CREATE TABLE` statements, or think
through your stored table structure ahead of time so you can name your field and
types. You simply put your objects into Convex and keep building your app!

However, moving fast early can be problematic later. "Was that field a number or
a string? I think I changed it when I fixed that one bug?"

Storage systems which are too permissive can sometimes become liabilities as
your system matures and you want to be able to reason assuredly about exactly
what data is in your system.

The good news is Convex is always typed. It's just implicitly typed! When you
submit a document to Convex, tracks all the types of all the fields in your
document. You can go to your [dashboard](/docs/dashboard.md) and view the
inferred schema of any table to understand what you've ended up with.

"What about that field I changed from a string to a number?" Convex can handle
this too. Convex will track those changes, in this case the field is a union
like `v.union(v.number(), v.string())`. That way even when you change your mind
about your documents fields and types, Convex has your back.

Once you are ready to formalize your schema, you can define it using our
[schema builder](/docs/database/schemas.mdx) to enable schema validation and
generate types based on it.
