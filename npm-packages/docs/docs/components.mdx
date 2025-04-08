---
title: "Components"
description: "Self contained building blocks of your app"
pagination_prev: search
---

import { ComponentCardList } from "@site/src/ComponentCardList.tsx";

<span className="convex-hero">
  Convex Components package up code and data in a sandbox that allows you to
  confidently and quickly add new features to your backend.
</span>

Convex Components are like mini self-contained Convex backends, and installing
them is always safe. They can't read your app's tables or call your app's
functions unless you pass them in explicitly.

You can read about the full vision in
[Convex: The Software-Defined Database](https://stack.convex.dev/the-software-defined-database#introducing-convex-components)

The Convex team has built a few components that add new features to your
backend. You'll eventually be able to author your own components to use within
your project and to share with the community, but we haven't stabilized and
documented the authoring APIs yet.

Each component is installed as its own independent library from NPM. Check out
the component's README for installation and usage instructions. You can see the
full directory on the [Convex website](https://convex.dev/components).

<CardLink
  className="convex-hero-card"
  item={{
    href: "https://convex.dev/components",
    label: "Full Components Directory",
  }}
/>

## Durable Functions

<ComponentCardList
  items={[
    {
      type: "link",
      href: "https://www.convex.dev/components/workflow",
      label: "Workflow",
      description: "Async code flow as durable functions.",
    },
    {
      type: "link",
      href: "https://www.convex.dev/components/workpool",
      label: "Workpool",
      description: "Async durable function queue.",
    },
    {
      type: "link",
      href: "https://www.convex.dev/components/crons",
      label: "Crons",
      description: "Dynamic runtime cron management",
    },
    {
      type: "link",
      href: "https://www.convex.dev/components/retrier",
      label: "Action Retrier",
      description: "Retry failed external calls automatically",
    },
  ]}
/>

## Database

<ComponentCardList
  items={[
    {
      type: "link",
      href: "https://www.convex.dev/components/sharded-counter",
      label: "Sharded Counter",
      description: "High-throughput counter operations",
    },
    {
      type: "link",
      href: "https://www.convex.dev/components/migrations",
      label: "Migrations",
      description: "Define and run migrations",
    },
    {
      type: "link",
      href: "https://www.convex.dev/components/aggregate",
      label: "Aggregate",
      description: "Efficient sums and counts",
    },
    {
      type: "link",
      href: "https://www.convex.dev/components/geospatial",
      label: "Geospatial (Beta)",
      description: "Store and search locations",
    },
  ]}
/>

## Integrations

<ComponentCardList
  items={[
    {
      type: "link",
      href: "https://www.convex.dev/components/cloudflare-r2",
      label: "Cloudflare R2",
      description: "Store and serve files",
    },
    {
      type: "link",
      href: "https://www.convex.dev/components/prosemirror-sync",
      label: "Collaborative Text Editor Sync",
      description: "Real-time collaborative text editing",
    },
    {
      type: "link",
      href: "https://www.convex.dev/components/push-notifications",
      label: "Expo Push Notifications",
      description: "Send mobile push notifications",
    },
    {
      type: "link",
      href: "https://www.convex.dev/components/twilio",
      label: "Twilio SMS",
      description: "Send and receive SMS messages",
    },
    {
      type: "link",
      href: "https://www.convex.dev/components/launchdarkly",
      label: "LaunchDarkly Feature Flags",
      description: "Sync feature flags with backend",
    },
    {
      type: "link",
      href: "https://www.convex.dev/components/polar",
      label: "Polar",
      description: "Add subscriptions and billing",
    },
  ]}
/>

## Backend

<ComponentCardList
  items={[
    {
      type: "link",
      href: "https://www.convex.dev/components/persistent-text-streaming",
      label: "AI Agent",
      description: "Define agents with tools and memory",
    },
    {
      type: "link",
      href: "https://www.convex.dev/components/persistent-text-streaming",
      label: "Persistent Text Streaming",
      description: "Stream and store text data",
    },
    {
      type: "link",
      href: "https://www.convex.dev/components/rate-limiter",
      label: "Rate Limiter",
      description: "Control resource usage rates",
    },
    {
      type: "link",
      href: "https://www.convex.dev/components/action-cache",
      label: "Action Cache",
      description: "Cache expensive external calls",
    },
  ]}
/>

<Admonition type="caution" title="The component authoring APIs are in Beta">
  The underlying authoring APIs for components are still in flux. The Convex
  team authored components listed below will be kept up to date as the APIs
  change.
</Admonition>

## Understanding Components

Components can be thought of as a combination of concepts from frontend
components, third party APIs, and both monolith and service-oriented
architectures.

### Data

Similar to frontend components, Convex Components encapsulate state and behavior
and allow exposing a clean interface. However, instead of just storing state in
memory, these can have internal state machines that can persist between user
sessions, span users, and change in response to external inputs, such as
webhooks. Components can store data in a few ways:

- Database tables with their own schema validation definitions. Since Convex is
  realtime by default, data reads are automatically reactive, and writes commit
  transactionally.
- File storage, independent of the main app's file storage.
- Durable functions via the built-in function scheduler. Components can reliably
  schedule functions to run in the future and pass along state.

Typically, libraries require configuring a third party service to add stateful
off-the-shelf functionality, which lack the transactional guarantees that come
from storing state in the same database.

### Isolation

Similar to regular npm libraries, Convex Components include functions, type
safety, and are called from your code. However, they also provide extra
guarantees.

- Similar to a third-party API, components can't read data for which you don't
  provide access. This includes database tables, file storage, environment
  variables, scheduled functions, etc.
- Similar to service-oriented architecture, functions in components are run in
  an isolated environment, so they can't read or write global variables or patch
  system behavior.
- Similar to a monolith architecture, data changes commit transactionally across
  calls to components, without having to reason about complicated distributed
  commit protocols or data inconsistencies. You'll never have a component commit
  data but have the calling code roll back.
- In addition, each mutation call to a component is a sub-mutation isolated from
  other calls, allowing you to safely catch errors thrown by components. It also
  allows component authors to easily reason about state changes without races,
  and trust that a thrown exception will always roll back the Component's
  sub-mutation. [Read more](/docs/components/using.mdx#transactions).

### Encapsulation

Being able to reason about your code is essential to scaling a codebase.
Components allow you to reason about API boundaries and abstractions.

- The transactional guarantees discussed above allows authors and users of
  components to reason locally about data changes.
- Components expose an explicit API, not direct database table access. Data
  invariants can be enforced in code, within the abstraction boundary. For
  example, the [aggregate component](https://convex.dev/components/aggregate)
  can internally denormalize data, the
  [rate limiter](https://convex.dev/components/rate-limiter) component can shard
  its data, and the
  [push notification](https://convex.dev/components/push-notifications)
  component can internally batch API requests, while maintaining simple
  interfaces.
- Runtime validation ensures all data that cross a component boundary are
  validated: both arguments and return values. As with normal Convex functions,
  the validators also specify the TypeScript types, providing end-to-end typing
  with runtime guarantees.
