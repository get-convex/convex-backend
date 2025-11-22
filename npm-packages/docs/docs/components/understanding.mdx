---
title: "Understanding Components"
slug: "understanding"
sidebar_position: 10
description: "Understanding components"
---

Convex components are self-contained backend modules that bundle functions,
schemas, and data together. They let you add complex functionality to your
app—like authentication, rate limiting, or document collaboration—without
implementing everything from scratch.

If you've worked with modern web development, you've likely encountered similar
ideas in different forms. Components draw inspiration from frontend components,
third-party APIs, and service-oriented architectures. The key difference is that
Convex components run within your backend, giving you the composability combined
with the persistence and reliability of backend services.

The following diagram shows how data and function access works in the component
ecosystem. Arrows from one element to another represent that an element has
access to the functions or data of the other element.

<p style={{ textAlign: "center" }}>
  <img
    src="/img/components-diagram.png"
    alt="Screenshot of the component dropdown"
    width={600}
  />
</p>

### Data

Similar to frontend components, Convex Components encapsulate state and behavior
and allow exposing a clean interface. However, instead of storing state in
memory, these can have internal state machines that can persist between user
sessions, span users, and change in response to external inputs, such as
webhooks. Components can store data in a few ways:

- Database tables with their own schema validation definitions. Since Convex is
  realtime by default, data reads are automatically reactive, and writes commit
  transactionally.
- File storage, independent of the main app's file storage.
- Durable functions via the built-in function scheduler. Components can schedule
  functions to run in the future and pass along state.

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
  an isolated environment, so writes to global variables and patches system
  behavior aren't shared between components.
- Similar to a monolith architecture, data changes commit transactionally across
  calls to components, without having to reason about complicated distributed
  commit protocols or data inconsistencies. You'll never have a component commit
  data but have the calling code roll back.
- In addition, each mutation call to a component is a sub-transaction isolated
  from other calls, allowing you to safely catch errors thrown by components. It
  also allows component authors to easily reason about state changes without
  races, and trust that a thrown exception will always roll back the Component's
  sub-transaction. [Read more](/components/using.mdx#transactions).

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
