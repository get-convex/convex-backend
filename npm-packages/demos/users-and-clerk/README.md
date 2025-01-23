# Users and Authentication Example App

This example demonstrates how to add users and authentication to a basic chat
app. It uses [Clerk](https://clerk.dev/) for authentication.

Users are initially presented with a "Log In" button. After user's log in, their
information is persisted to a `users` table. When users send messages, each
message is associated with the user that sent it. Lastly, users can log out with
a "Log Out" button.

## Running the App

```sh
npm run dev
```

### Using your own Clerk instance

Follow the instructions in https://docs.convex.dev/auth/clerk#get-started to
obtain:

- A _publishable key_, use it in `main.tsx`
- A JWT template Issuer URL, use it in `auth.config.ts`.
