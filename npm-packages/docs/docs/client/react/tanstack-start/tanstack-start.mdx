---
title: "TanStack Start"
sidebar_label: "TanStack Start"
sidebar_position: 180
description: "How Convex works with TanStack Start"
---

[TanStack Start](https://tanstack.com/start/latest) is a new React web framework
with best-in-class typesafe routing.

When used with Convex, TanStack Start provides

- Live-updating queries with React Query (the React client for TanStack Query)
- Subscription session resumption, from SSR to live on the client
- Loader-based preloading and prefetching
- Consistent logical query timestamp during SSR
- Opt-in component-local SSR

and more!

This page describes the recommended way to use Convex with TanStack Start, via
React Query. The standard Convex React hooks work also with TanStack Start
without React Query, as do the
[React Query hooks](/docs/client/tanstack-query.mdx) without TanStack Start! But
using all three is a sweet spot.

<Admonition type="caution" title="TanStack Start is in Beta">
  TanStack Start is a new React framework currently in beta. You can use it
  today but there may be breaking changes made to it before a stable release.
</Admonition>

## Getting started

Follow the [TanStack Start Quickstart](/docs/quickstart/tanstack-start.mdx) to
add Convex to a new TanStack Start project.

## Using Convex with React Query

You can read more about [React Query hooks](/docs/client/tanstack-query.mdx),
but a few highlights relevant to TanStack Start.

### Staying subscribed to queries

Convex queries in React Query continue to receive updates after the last
component subscribed to the query unmounts. The default for this behavior is 5
minutes and this value is configured with
[`gcTime`](https://tanstack.com/query/latest/docs/framework/react/guides/caching).

This is useful to know when debugging why a query result is already loaded: for
client side navigations, whether a subscription is already active can depend on
what pages were previously visited in a session.

### Using Convex React hooks

[Convex React](/docs/client/react.mdx) hooks like
[`usePaginatedQuery`](/api/modules/react#usepaginatedquery) can be used
alongside TanStack hooks. These hooks reference the same Convex Client so
there's still just one set of consistent query results in your app when these
are combined.

## Server-side Rendering

Using TanStack Start and Query with Convex makes it particularly easy to
live-update Convex queries on the client while also
[server-rendering](https://tanstack.com/query/v5/docs/framework/react/guides/ssr)
them.
[`useSuspenseQuery()`](https://tanstack.com/query/latest/docs/framework/react/reference/useSuspenseQuery)
is the simplest way to do this:

```ts
const { data } = useSuspenseQuery(convexQuery(api.messages.list, {}));
```

### Consistent client views

In the browser all Convex query subscriptions present a consistent,
at-the-same-logical-timestamp view of the database: if one query result reflects
a given mutation transaction, every other query result will too.

Server-side rendering is usually a special case: instead of a stateful WebSocket
session, on the server it's simpler to fetch query results ad-hoc. This can lead
to inconsistencies analogous to one REST endpoint returning results before a
mutation ran and another endpoint returning results after that change.

In TanStack Start, this issue is avoided by sending in a timestamp along with
each query: Convex uses the same timestamp for all queries.

### Loaders

To make client-side navigations faster you can add a
[loader](https://tanstack.com/router/latest/docs/framework/react/guide/external-data-loading#using-loaders-to-ensure-data-is-loaded)
to a route. By default, loaders will run when mousing over a link to that page.

```ts
export const Route = createFileRoute('/posts')({
  loader: async (opts) => {
    await opts.context.queryClient.ensureQueryData(
      convexQuery(api.messages.list, {}),
    );
  };
  component: () => {
    const { data } = useSuspenseQuery(convexQuery(api.messages.list, {}));
    return (
      <div>
	{data.map((message) => (
	  <Message key={message.id} post={message} />
	))}
      </div>
    );
  },
})
```

## Authentication

Client-side authentication in Start works the way
[client-side authentication with Convex](https://docs.convex.dev/auth) generally
works in React because TanStack Start works well as a client-side framework.

To use Clerk auth to make authenticated Convex calls on the server as well see
the [TanStack Start + Clerk guide](/client/react/tanstack-start/clerk.mdx).

Clerk is an official partner of TanStack, see our setup guide.
