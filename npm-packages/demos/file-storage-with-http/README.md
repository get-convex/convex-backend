# File Storage with HTTP Actions

This example demonstrates how to use Convex file storage to augment Convex Chat
with images via [HTTP actions](https://docs.convex.dev/functions/http-actions).

This app is an extension of the Convex chat tutorial including a new button for
uploading images. Images will be stored in Convex file storage, with their
storage IDs saved in the messages table for access.

To learn more about storage see the
[File Storage](https://docs.convex.dev/file-storage) documentation.

## Running the App

### 1. Start dev

```
npm install
npm run dev
```

### 2. Add `VITE_CONVEX_SITE_URL` variable

Afterwards add `VITE_CONVEX_SITE_URL` to your `.env.local` file, by copying
`VITE_CONVEX_URL` and changing the top-level domain from `cloud` to `site`, like
this:

```
VITE_CONVEX_URL="https://happy-animal-123.convex.cloud"
VITE_CONVEX_SITE_URL="https://happy-animal-123.convex.site"
```

### 3. Set `CLIENT_ORIGIN` variable

Also add a `CLIENT_ORIGIN`
[Convex environment variable](https://docs.convex.dev/production/environment-variables)
on your [Convex dashboard](https://dashboard.convex.dev/) with the origin of
your website (e.g. `http://localhost:5173` if developing locally).
