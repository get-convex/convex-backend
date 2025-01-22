# Users and Authentication Example App

This example demonstrates how to add users and authentication to a basic chat
app. It uses [Auth0](https://auth0.com/authentication) for authentication.

Users are initially presented with a "Log In" button. After user's log in, their
information is persisted to a `users` table. When users send messages, each
message is associated with the user that sent it. Lastly, users can log out with
a "Log Out" button.

## Running the App

Because this app uses authentication, it requires a bit of an additional setup.

Follow these instructions https://docs.convex.dev/auth/auth0 to set up Auth0
with Convex. You will have to update the client in `main.tsx` and the server in
`auth.config.js`.
