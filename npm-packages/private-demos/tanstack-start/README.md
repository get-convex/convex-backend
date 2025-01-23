# Using Convex with TanStack Start

Using Convex with TanStack Start provides

- Live-updating queries: updates come in over a WebSocket instead of requiring polling
- Works with TanStack Query `useQuery`, useSuspenseQuery`, etc. hooks
- Automatic query invalidation: when a mutation succeeds all queries it affects update automatically
- Selective optimistic update rollback: when a mutation succeeds only its update will be rolled back, with other optimistic updates reapplied
- Consistent snapshot reads of database state: /messages will never return a foreign key for a /user that doesn't exist until the next fetch

# Examples

### Sibling component calls

During SSR two sibling components make requests at about the same time.

### Subsequent useSuspenseQuery calls

During SSR one component makes one query and then another, as though the second
depended on the first and it was a dependent query (a "waterfall.")
