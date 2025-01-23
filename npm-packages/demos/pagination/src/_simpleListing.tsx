// This file is not used in the demo app.
// It showcases only the basic pagination call.

// @snippet start example
import { usePaginatedQuery } from "convex/react";
import { api } from "../convex/_generated/api";

export function App() {
  const { results, status, loadMore } = usePaginatedQuery(
    api.messages.list,
    {},
    { initialNumItems: 5 },
  );
  return (
    <div>
      {results?.map(({ _id, text }) => <div key={_id}>{text}</div>)}
      <button onClick={() => loadMore(5)} disabled={status !== "CanLoadMore"}>
        Load More
      </button>
    </div>
  );
}
// @snippet end example
