# Using Convex with Node.js

Convex WebSocket and HTTP clients can be used from Node.js.

Node.js scripts can be used to automate administrative tasks and long-running
Node.js program or servers can monitor a query and take action whenever the
query results update.

## Using Convex with Node.js with codegen

If your JavaScript code uses import/export syntax, calling Convex functions
works just like in a browser.

```js
import { ConvexHttpClient } from "convex/browser";
import { api } from "./convex/_generated/api.js";

const client = new ConvexHttpClient(CONVEX_URL_GOES_HERE);
client.query(api.messages.list).then(console.log);
```

_TypeScript_

If you use TypeScript to _compile_ your code (this is rare for web projects but
more common with Node.js), add `"allowJs": true` to tsconfig.json compiler
options so that TypeScript will compile the `api.js` file as well.

Just like bundling for the browser, bundling TypeScript code for Node.js with
webpack, esbuild, rollup, vite, and others usually allow you import from code
that uses import/export syntax with no extra setup.

_TypeScript with ESM without a compile step_

If you want to run your TypeScript script directly without a compile step,
installing [tsx](https://www.npmjs.com/package/tsx) and running your script with
tsx should work.

_CommonJS (require syntax)_

If you don't use `"type": "module"` in the package.json of your project you'll
need to use `require()` syntax and Node.js will not be able to import the
`convex/_generated/api.js` file directly.

In the same directory as your package.json, create or edit convex.json with

```json
{
  "generateCommonJSApi": true
}
```

When you run codegen (with `npx convex dev`) a new `api_cjs.cjs` file will be
created which can be imported from CommonJS code.

```js
const { ConvexHttpClient, ConvexClient } = require("convex/browser");
const { api } = require("./convex/_generated/api_cjs.cjs");
const httpClient = new ConvexHttpClient(CONVEX_URL_GOES_HERE);
```

_TypeScript with CommonJS without a compile step_

Follow the steps above for CommonJS and use
[`tsx`](https://www.npmjs.com/package/tsx) to run you code. Be sure your
tsconfig.json is configured for CommonJS output.

## Using Convex with Node.js without codegen

You can always use strings if you don't have the Convex functions and api file
handy. An api reference like `api.folder.file.exportName` becomes
`"folder/file:exportName"`.

## Running the App

First deploy the backend with `npx convex dev`.

Then try these scripts:

```
node script.js
node script.cjs
node tsx script-in-typescript.ts
```
