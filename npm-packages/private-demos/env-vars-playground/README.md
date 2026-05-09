## Safe Environment Variable Playground

You can experiment with how defining safe environment variables work in this
demo.

Modify the `convex/convex.config.ts` file to add or remove declared env vars and
`convex/functions.ts` to make use of them.

You can also try changing `fakecomponent/convex.config.ts` which the app hooks
up.

Spin up a backend using `just run-local-backend` and try your changes with
`just convex dev`.

You can also try adding/deleting/changing environment variables on the dashboard
after starting it with `just run-dashboard-local-backend`.
