# Sessions Example App

This example demonstrates using a pattern to keep track of user sessions in a
database table to track per-tab or per-browser data, without being logged in.

It leverages [`convex-helpers`](https://www.npmjs.com/package/convex-helpers) in
[sessions.ts](./convex/lib/sessions.js) to wrap Convex
[functions](https://docs.convex.dev/using/writing-convex-functions) and
[useSession.ts](./src/useSession.ts) to wrap the `useQuery` and `useMutation`
hooks in React.

More detail can be found in the
[Stack post](https://stack.convex.dev/sessions-wrappers-as-middleware).

## Using sessions yourself:

1. In addition to a `ConvexProvider`, wrap your app with a `SessionProvider`:

   ```
   <ConvexProvider client={convex}>
     <SessionProvider>
       <App />
     </SessionProvider>
   </ConvexProvider>
   ```

2. Use `queryWithSession` to define your function:

   ```
   export const myQuery = queryWithSession({
     args: {},
     handler: async (ctx, args) => {
      console.log(ctx.session._id);
       ...
     },
   });
   ```

3. Use `useSessionQuery` in your React client:

   ```
   const messages = useSessionQuery(api.myModule.myQuery);
   ...
   ```

   Note: the same utilities are available for mutations & actions.

4. [Optional] Write any data that you want to be available in subsequent session
   requests to the `sessions` table :
   ```
   db.patch(session._id, { userId });
   ```

## Running the App

Run:

```
npm install
npm run dev
```
