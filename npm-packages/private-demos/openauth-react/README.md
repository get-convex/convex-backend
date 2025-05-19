# OpenAuth with Convex Custom JWT Auth Demo

This is an example of running OpenAuth server(s) to authenticate Convex
WebSocket connections to a Convex deployment.

This example only works against a local backend because server that hosts the
.well-known/jwks.json endpoint that the Convex deployment must hit is hosted
locally in this example at localhost:3000.

A setup that worked for production or cloud development would look similar but
with these OpenAuth servers hosted at domains accessible from the public
internet.

---

You need three servers to run this demo. The first is the web UI:

```bash
bun run dev
```

Then visit `http://localhost:5173` in your browser.

You'll need to run Convex; in the monorepo, run

```bash
just run-local-backend
```

It needs the OpenAuth server running at `http://localhost:3000`. Start it with

```bash
bun run --hot issuer.ts
```

Start a JWT API running to get the user subject on `http://localhost:3001`.

```bash
bun run --hot jwt-api.ts
```
