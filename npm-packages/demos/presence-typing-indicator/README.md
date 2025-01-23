# Presence: Typing indicator

This example app demonstrates how to use Convex for ephemeral data, in this case
to convey another user's typing status. It stores the "presence" data in a
presence table, which it updates periodically so other clients know it's active.

## `usePresence`

It can store arbitrary presence data using the
[`usePresence`](./src/hooks/usePresence.js) hook.

```js
const [myPresence, othersPresence, updateMyPresence] = usePresence(
  "chat-room",
  name,
  { typing: false },
);
```

This hard-codes a single "room", but you could easily extend it to pass a chat
channel as the "room" to have presence per-channel.

Note: it ignores the first return argument, which is this user's presence data.
In this example, we don't need to know whether the user is typing.

### Updating Presence Data

In this case, it updates whether the user is typing with a `useEffect`:

```js
useEffect(() => {
  if (newMessageText.length === 0) {
    updateMyPresence({ typing: false });
    return;
  }
  updateMyPresence({ typing: true });
  const timer = setTimeout(() => updateMyPresence({ typing: false }), 1000);
  return () => clearTimeout(timer);
}, [updateMyPresence, newMessageText]);
```

It waits 1s after the last change to the message text to set `typing` to `false`
and sets it to `false` immediately when the text is empty (e.g. after sending a
text).

### Using Presence Data

It uses the list of presence data for other users to show who is typing. The
user's data is excluded from this list.

```jsx
(othersPresence ?? [])
  .filter(({ data, updated }) => data.typing && Date.now() - updated < OLD_MS)
  .map(({ user }) => (
    <li key={user}>
      <span>{user}</span>
      <span>
        <i>typing...</i>
      </span>
    </li>
  ));
```

Like the `useQuery` Convex hook, `othersPresence` will be `undefined` until it
receives the first response from Convex.

It filters to just users who are typing, and filters out users who haven't been
updated in a set amount of time, in case a user left the page before their timer
fired to set `typing` to `false`.

The `usePresence` hook sends periodic "heartbeats" to bump "updated".

### Under the hood

See [`usePresence`](./src/hooks/usePresence.js) for more details.

Presence data is merged, so you can update `{typing: true}` in one place and
`{other: data}` elsewhere and the data will be sent up with the latest values of
`typing` and `other`.

`usePresence` uses [`useSingleFlight`](./src/hooks/useSingleFlight.js) to
throttle its requests to the server, which means not all incremental updates
will be delivered, but the latest presence data will eventually be synced.

## Running the App

Run:

```
npm install
npm run dev
```
