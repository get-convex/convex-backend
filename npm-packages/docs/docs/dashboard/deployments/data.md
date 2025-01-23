---
title: "Data"
slug: "data"
sidebar_position: 5
---

![Data Dashboard Page](/screenshots/data.png)

The [data page](https://dashboard.convex.dev/deployment/data) allows you to view
and manage all of your tables and documents.

On the left side of the page is a list of your tables. Clicking on a table will
allows you to create, view, update, and delete documents in that table.

You may drag-and-drop the column headers in each table to visually re-order the
data.

A readonly view of the data page is available in the
[command line](/docs/cli.md#display-data-from-tables).

```sh
npx convex data [table]
```

## Filtering documents

You may filters documents on the data page by clicking the "Filter" button on
the top of the page.

![Data filters](/screenshots/data_filters.png)

All fields in a document are filterable by the operations supported in Convex
query syntax.
[Equality](/docs/database/reading-data/reading-data.mdx#equality-conditions) and
[Comparison](/docs/database/reading-data/reading-data.mdx#comparisons) when
filtering in the dashboard share the same rules as a query using the Convex
client. You may also filter based on the type of the field.

To add a filter, click the `+` next to an existing filter. If you add more than
one condition, they will be evaluated using the `and` operation.

For each filter, you must select a field to filter by, operation, and comparison
value. In the third input box (selecting a value), you may enter a valid Convex
value, such as `"a string"`, `123`, or even a complex object, such as
`{ a: { b: 2 } }`

<Admonition type="note">

When filtering by `_creationTime`, a date picker will be displayed instead of
the normal JavaScript syntax input box. Comparisons for `_creationTime` are made
at the nanosecond granularity, so if you'd like to filter to an exact time, try
adding two filter conditions for `creationTime >= $time` and
`creationTime <= $time + 1 minute`.

</Admonition>

## Writing custom queries

You can write a [query](/docs/database/reading-data/reading-data.mdx) directly
in the dashboard. This allows you to perform arbitrary filtering and
transformation of the data, including sorting, joins, grouping and aggregations.

In the `⋮` overflow menu at the top of the data page click on the “Custom query”
option.

<img
    src="/screenshots/data_custom_query.png"
    alt="Custom query button"
    width={250}
/>

This opens the same UI used for
[running your deployed functions](/docs/dashboard/deployments/functions.md#running-functions),
but with the “Custom test query” option selected, which lets you edit the source
code for the query. This source code will be sent to your deployment and
executed when you click on the “Run Custom Query“ button.

![Running a custom test query](/screenshots/data_custom_query_runner.png)

If you're not on the data page, you can still open this UI via the persistent
_fn_ button shown on the bottom right of all deployment pages. The keyboard
shortcut to open the function runner is Ctrl + ` (backtick).

## Creating tables

You may create a table from the dashboard by clicking the "Create Table" button
and entering a new name for the table.

## Creating documents

You may add individual documents to the table using the “Add Documents” button
located in the data table's toolbar.

Once you click “Add Documents” a side panel will open, allowing you to add new
documents to your table using JavaScript syntax. To add more than one document
add a time, add new objects to the array in the editor.

![Add document](/screenshots/data_add_document.png)

## Quick actions (context menu)

You can right-click on a document or value to open a context menu with quick
actions, like copying values, quickly filtering by the selected value, and
deleting documents.

![Quick actions context menu](/screenshots/data_context_menu.png)

## Editing a cell

To edit a cell's value, double-click on the cell in the data table, or press the
Enter key while it’s selected. You can change the selected cell by using the
arrow keys.

You can change the value by editing inline, and pressing enter to save.

<Admonition type="note">

You can even edit the type of your value here, as long as it satisfies your
[schema](/docs/database/schemas.mdx) — try replacing a string with an object!

</Admonition>

![Inline value editor](/screenshots/data_edit_inline.png)

## Editing a document

To edit multiple fields in a document at the same time, hover over the document
and right-click to open the context menu. From there you can click on "Edit
Document".

![Edit entire document](/screenshots/data_edit_document.png)

## Adding references to other documents

To reference another document, use the string ID of the document you want to
reference.

You can copy the ID by clicking on its cell and pressing CTRL/CMD+C.

## Bulk editing documents

You can edit multiple or all documents at once. To select all documents click on
the checkbox in the table header row. To select individual documents hover over
the left-most cell and click the checkbox that appears. To select multiple
adjacent documents at once, press the Shift key when clicking on the checkbox.

When at least one document is selected, the “(Bulk) Edit Document(s)” button
will be visible in the table toolbar. Click the button and an editor will appear
on the right hand side.

![Bulk edit documents](/screenshots/data_bulk_edit.png)

## Deleting documents

When at least one document is selected (see above), the “Delete Document(s)”
button will be visible in the table toolbar. Click the button to delete
documents. If you're editing data in a production deployment a confirmation
dialog will appear before the documents are deleted.

## Clear a table

You can also delete all documents by clicking on the `⋮` overflow menu at the
top of the data page and clicking "Clear Table". This action will delete all
documents in the table, without deleting the table itself.

In production environments, the Convex dashboard will have you type in the name
of the table before deletion.

## Delete a table

<Admonition type="caution" title="This is a permanent action">

Deleting a table is irreversible. In production environments, the Convex
dashboard will have you type in the name of the table before deletion.

</Admonition>

The "Delete table" button can be found by clicking on the `⋮` overflow menu at
the top of the data page. This action will delete all documents this table, and
remove the table from your list of tables. If this table had indexes, you will
need to redeploy your convex functions (by running `npx convex deploy` or
`npx convex dev` for production or development, respectively) to recreate the
indexes.

## Generating a schema

At the bottom-left of the page is a "Generate Schema" button which you can click
to have Convex generate a [schema](/docs/database/schemas.mdx) of all your
documents within this table.

![Generate Schema button](/screenshots/data_generate_schema.png)

## Table Schema and Indexes

The "Schema and Indexes" button can be found by clicking on the `⋮` overflow
menu at the top of the data page.

This button will open a panel showing the saved
[schema](/docs/database/schemas.mdx) and
[indexes](/docs/database/reading-data/indexes/indexes.md) associated with the
selected table.

Indexes that have not completed backfilling will be accompanied by a loading
spinner next to their name.

![Table indexes](/screenshots/data_indexes.png)
