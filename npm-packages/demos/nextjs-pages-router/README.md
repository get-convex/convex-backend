# Next.js Example App

This is a [Next.js](https://nextjs.org/) project bootstrapped with
[`create-next-app`](https://github.com/vercel/next.js/tree/canary/packages/create-next-app).

```bash
npx create-next-app@latest --ts
```

After creating the project, convex was installed:

```bash
npm install convex
```

Convex functions were added to the `convex/` directory and a provider was added
to `_app.tsx`.

To learn more about using Convex with Next.js, see the
[Convex Next.js documentation](https://docs.convex.dev/client/react/nextjs)!

## Running the App

This demo uses Auth0 for authentication. To set it up follow the
[Convex Auth0](https://docs.convex.dev/auth/auth0) documentation. Instead of
hardcoding the `domain` and `clientId` in `_app.jsx` you can add them to the
`.env` file:

```
NEXT_PUBLIC_AUTH0_DOMAIN = "<your domain>.us.auth0.com"
NEXT_PUBLIC_AUTH0_CLIENT_ID = "<your client id>"
```

You can then run

```
npm install
npm run dev
```

Then navigate to http://localhost:3000 in your browser.
