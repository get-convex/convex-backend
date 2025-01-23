# Vector Search Example App

This example demonstrates how to use
[Convex vector search](https://docs.convex.dev/vector-search).

It has a "Food search" and "Movie search". The "Food search" shows the simplest
way to set up a vector search, and matches the Convex
[documentation](https://docs.convex.dev/vector-search).

The "Movies search" shows some alternative pattens documented
[here](https://docs.convex.dev/vector-search#advanced-patterns).

## Running the App

### 1. Start dev

Run:

```
npm install
npm run dev
```

### 2. Add `OPENAI_KEY` variable

This app uses OpenAI to generate embeddings. Add `OPENAI_KEY`
[Convex environment variable](https://docs.convex.dev/production/environment-variables)
on your [Convex dashboard](https://dashboard.convex.dev/) with your OpenAI API
key.
