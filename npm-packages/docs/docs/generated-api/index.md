---
title: Generated Code
description:
  "Auto-generated JavaScript and TypeScript code specific to your app's API"
---

Convex uses code generation to create code that is specific to your app's data
model and API. Convex generates JavaScript files (`.js`) with TypeScript type
definitions (`.d.ts`).

Code generation isn't required to use Convex, but using the generated code will
give you more better autocompletion in your editor and more type safety if
you're using TypeScript.

To generate the code, run:

```
npx convex dev
```

This creates a `convex/_generated` directory that contains:

- [`api.js` and `api.d.ts`](./api.md)
- [`dataModel.d.ts`](./data-model.md)
- [`server.js` and `server.d.ts`](./server.md)
