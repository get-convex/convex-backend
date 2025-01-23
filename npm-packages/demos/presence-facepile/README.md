# Presence: Facepile

This example app demonstrates how to use Convex for ephemeral data, in this case
to convey another user's presence with a user-selectable emoji in a "facepile".
It stores the "presence" data in a presence table, which it updates periodically
so other clients know it's active.

## `usePresence`

It can store arbitrary presence data using the
[`usePresence`](./src/hooks/usePresence.js) hook.

```js
const [myPresence, othersPresence, updateMyPresence] = usePresence(
  "chat-room",
  name,
  {
    name,
    emoji: initialEmoji,
  },
);
```

This hard-codes a single "room", but you could easily extend it to pass a chat
channel as the "room" to have presence per-channel.

It is using the user's name as the user identifier. To make this more secure,
you could use auth (see the "users-and-auth" demo) and not rely on passing the
user's identifier from the client.

### Updating Presence Data

In this case, it updates the user's emoji in a callback from a `<select>`:

```jsx
<select
    defaultValue={myPresence.emoji}
    onChange={e => updateMyPresence({ emoji: e.target.value })}
>
```

Note that this merges with the existing data, so `name` is unchanged.

### Using Presence Data

It uses the list of presence data for other users to show who is active on a
page. The user's data is excluded from this list.

```jsx
<FacePile othersPresence={othersPresence ?? []} />
```

Like the `useQuery` Convex hook, `othersPresence` will be `undefined` until it
receives the first response from Convex.

The `FacePile` will show old users as greyed out, and on hover will say when
they last updated.

The `usePresence` hook sends periodic "heartbeats" to bump "updated".

### `FacePile`

In order to keep the [`FacePile`](./src/Facepile.jsx) updated, it uses
`useState` for the time:

```js
const [now, setNow] = useState(Date.now());
useEffect(() => {
  const intervalId = setInterval(() => setNow(Date.now()), UPDATE_MS);
  return () => clearInterval(intervalId);
}, [setNow]);
```

It then uses `now` to determine which users are old:

```js
othersPresence.map((presence) => ({
  ...presence,
  old: presence.updated < now - OLD_MS,
}));
```

And sorts users & displays them as greyed out using that value.

### Under the hood

See [`usePresence`](./src/hooks/usePresence.js) for more details.

Presence data is merged, so you can update `{emoji}` in one place and `{name}`
elsewhere and the data will be sent up with the latest values of `emoji` and
`name`.

`usePresence` uses [`useSingleFlight`](./src/hooks/useSingleFlight.js) to
throttle its requests to the server, which means not all incremental updates
will be delivered, but the latest presence data will eventually be synced.

## Running the App

Run:

```
npm install
npm run dev
```
