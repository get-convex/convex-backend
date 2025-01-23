# Middleware

Writing middleware in JavaScript is pretty easy, but writing middleware that
preserves types is tricky. We implement some middleware in the monorepo to
better understand the effect of changes in client APIs, in particular types.

# Types of middleware

Implementing custom behavior in JavaScript is pretty easy, it's preserving the
types that makes this difficult.

### Custom hooks in React

Now that useQuery is not generated code custom hooks should be reusable.

The tricky bit is dealing with the variadicity of these hooks in TypeScript.

```ts
let x = useQuery("listMessages");
let x = useQuery("listMessages", {}); // allowed
let x = useQuery("listMessages", undefined); // allowed
let x = useQuery("listMessagesForChannel", { channel: 17 });
let x = useQuery("listMessagesForChannel"); // type error
```

We have two options for implementing these:

- Overload (write two more specific signatures for) `useQuery<QueryReference>`
  for queries with empty and non-empty args
- Rest arg that may have one or zero elements

Both of these work poorly. We have these type tests here for comparing them.

We could encourage custom hooks to wrap the non-variadic `useQueries` instead,
applying -- although this breaks composition.

We could demonstrate how to cheat the types; it's ok if types break when
_writing_ middleware as long as the experience of _using_ the hooks is good.

### Custom wrappers on the backend

Our code-generated wrappers like `query`, `mutation`, and `action` should be
able to be extended and these extensions should be composeable. Middleware
should be able to:

- modify the .input validator (influencing the type of the handler)
- add an input validator if one was not already set
- write code that wraps the function (wraps it in a try/catch, modifies input
  and output, runs it twice, whatever)
- access the ctx (e.g. validate that a DB record exists) in the before and after
  code
- write code that modifies the result of the function
- compose arbitrarily other middleware

Maybe we want other things

- set new metadata on the function?

Complicated things like

```ts
import { mutation } from "./_generated/server";
const myMutWrapper = withSession(withUser(withCustomerCtx(mutation)))
export myMut = myMutWrapper({
  input: { a: v.string() },
  openAPIexample: "Run the function like this."
  customContext: { foo: 123 },
  handler: ({ user, session, foo }, { a, addedByAWrapper }) => { ... }
}]
```

There are a few ways to wrap Convex functions:

```ts
wrapTheImpl = mutation(modifyTheFunction((ctx, { a: number }) => {}));
wrapTheImpl2 = mutation({
  args: { a: v.number() },
  handler: modifyTheFunction((ctx, { a: number }) => {})
}
wrapTheMutation = modifyTheMutation(mutation((ctx, { a: number }) => {}));
wrapTheMutation2 = modifyTheMutation(mutation({
  args: { a: v.number() },
  handler: (ctx, { a: number }) => {}
}
wrapTheWrapper = modifyTheMutation(mutation)((ctx, { a: number }) => {});
```

We should probably choose one of these to endorse.

Ian say's it's been convenient to run wrappers at definition site instead of
using the `wrapTheWrapper` approach so he can mix and match middleware.

`wrapTheImpl2` isn't generally as powerful: you can't modify args and other
metadata with it.

I wonder if with the `wrapTheMutation` approach it's even possible to influence
the inferred signature of the function. It looks like it wouldn't be? But you
could annotate the return type of mutation() and that could do it.

Ian reports that the `wrapTheMutation1` approach fails to infer the context
type, but `wrapTheMutation2` is fine.

That leaves wrapTheMutation1 and wraptheImpl1.

# Instructions for writing middleware?
