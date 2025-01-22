import { usePaginatedQuery } from "convex/react";
import { api } from "../convex/_generated/api";

export default function App() {
  const numbers = usePaginatedQuery(
    api.numbers.numberPage,
    {},
    { initialNumItems: 200 },
  );

  return (
    <main>
      <h1>Find the biggest number</h1>
      <ul>
        {numbers.results.map((number, i) => (
          <li key={i}>
            <span>{number}</span>
          </li>
        ))}
      </ul>
      <p>Status {numbers.status}</p>
      <button onClick={() => numbers.loadMore(200)}>Load More</button>
    </main>
  );
}
