// This file is not used in the demo app.
// It showcases only the basic pagination call.

// @snippet start example
import { usePaginatedQuery } from "convex/react";
import { api } from "../convex/_generated/api";

export function App() {
  const { data, canLoadMore, loadMore } = usePaginatedQuery({
    query: api.messages.list,
    args: {},
    initialNumItems: 5,
  });
  return (
    <div>
      {data?.map(({ _id, body }) => <div key={_id}>{body}</div>)}
      <button onClick={() => loadMore(5)} disabled={!canLoadMore}>
        Load More
      </button>
    </div>
  );
}
// @snippet end example
