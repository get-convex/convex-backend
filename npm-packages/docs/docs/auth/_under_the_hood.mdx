The authentication flow looks like this under the hood:

1. The user clicks a login button
2. The user is redirected to a page where they log in via whatever method you
   configure in {props.provider}
3. After a successful login {props.provider} redirects back to your page, or a
   different page which you configure via {props.configProp}.
4. The {props.providerProvider} now knows that the user is authenticated.
5. The {props.integrationProvider} fetches an auth token from {props.provider}.
6. The `ConvexReactClient` passes this token down to your Convex backend to
   validate
7. Your Convex backend retrieves the public key from {props.provider} to check
   that the token's signature is valid.
8. The `ConvexReactClient` is notified of successful authentication, and
   {props.integrationProvider} now knows that the user is authenticated with
   Convex. `useConvexAuth` returns `isAuthenticated: true` and the
   `Authenticated` component renders its children.

{props.integrationProvider} takes care of refetching the token when needed to
make sure the user stays authenticated with your backend.
