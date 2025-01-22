# Search Example App

This example demonstrates how to use
[Convex full text search](https://docs.convex.dev/text-search) to add search to
an app.

In `schema.ts`, we define a search index on the `"messages"` table.
`searchMessages.js` uses this index to find all messages that match a search
query. This is all wired up to the front end in `App.jsx`.

## Running the App

Run:

```
npm install
npm run dev
```
