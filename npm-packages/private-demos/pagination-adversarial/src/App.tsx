import { useMutation, usePaginatedQuery } from "convex/react";
import { api } from "../convex/_generated/api";
import { FormEvent } from "react";
import { useEffect } from "react";
import { OptimisticLocalStore } from "convex/browser";
import { useState } from "react";
import { FunctionArgs } from "convex/server";
import { insertAtPosition } from "convex/react";
import { compareValues } from "convex/values";

export default function App() {
  const [tab, setTab] = useState<"filteredNumbers" | "optimisticUpdates">(
    "filteredNumbers",
  );

  return (
    <main>
      <button
        style={{
          textDecoration: tab === "filteredNumbers" ? "underline" : "none",
        }}
        onClick={() => setTab("filteredNumbers")}
      >
        Filtered Numbers
      </button>
      <button
        style={{
          textDecoration: tab === "optimisticUpdates" ? "underline" : "none",
        }}
        onClick={() => setTab("optimisticUpdates")}
      >
        Optimistic Updates
      </button>
      {tab === "filteredNumbers" && <FilteredNumbers />}
      {tab === "optimisticUpdates" && <OptimisticUpdates />}
    </main>
  );
}

function FilteredNumbers() {
  const numbers = usePaginatedQuery(
    api.numbers.listFilteredNumbers,
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

function insertNumberOptimisticUpdate(
  sortOrder: "asc" | "desc",
  localQueryStore: OptimisticLocalStore,
  args: FunctionArgs<typeof api.numbers.insert>,
) {
  const newNumber = {
    _id: "optimistic" as any,
    id: args.id,
    _creationTime: Date.now(),
    number: args.number,
    color: "blue",
    isStart: false,
    isEnd: false,
  };
  insertAtPosition({
    paginatedQuery: api.numbers.listNumbers,
    argsToMatch: { sortOrder },
    sortOrder,
    sortKeyFromItem: (e) => [e.number, e._creationTime],
    localQueryStore,
    item: newNumber,
  });
}

function useOptimisticUpdate(sortOrder: "asc" | "desc") {
  return useMutation(api.numbers.insert).withOptimisticUpdate(
    (localQueryStore, args) => {
      insertNumberOptimisticUpdate(sortOrder, localQueryStore, args);
    },
  );
}

function OptimisticUpdates() {
  const [sortOrder, setSortOrder] = useState<"asc" | "desc">("asc");
  const resetData = useMutation(api.numbers.reset);
  const [resetCounter, setResetCounter] = useState(0);
  const [intervalMs, setIntervalMs] = useState<number>(1000);
  const [isPaused, setIsPaused] = useState(true);
  const [lastNumber, setLastNumber] = useState<number | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [double, setDouble] = useState(false);
  const [showPageBoundaries, setShowPageBoundaries] = useState(false);
  const addNumber = useOptimisticUpdate(sortOrder);

  useEffect(() => {
    if (isPaused) {
      return;
    }
    if (intervalMs <= 100) {
      setError("Interval must be greater than 100");
      return;
    }
    const sendMessageInterval = setInterval(() => {
      const number = Math.floor(Math.random() * 100);
      setLastNumber(number);
      addNumber({
        number,
        id: crypto.randomUUID(),
      }).catch((e) => {
        console.error("Error while sending message", e);
        setError(e.message);
      });
    }, intervalMs);
    return () => clearInterval(sendMessageInterval);
  }, [intervalMs, isPaused, addNumber]);
  return (
    <main style={{ display: "flex", flexDirection: "column", gap: "10px" }}>
      <details>
        <summary>What is this?</summary>
        <p>
          This is a demo of optimistic updates on paginated queries. This shows
          a list of numbers sorted by their value.
        </p>
        <p>
          Optimistic updates are shown in blue. Try slowing down your network to
          see the optimistic updates for longer.
        </p>
      </details>
      <div style={{ display: "flex", gap: "10px", alignItems: "baseline" }}>
        <div
          style={{
            display: "flex",
            gap: "10px",
            flexDirection: "column",
            border: "1px solid black",
            width: "100%",
          }}
        >
          {error && (
            <div style={{ color: "red" }}>
              {error}
              <button onClick={() => setError(null)}>Clear</button>
            </div>
          )}
          <div style={{ display: "flex", gap: "10px", alignItems: "baseline" }}>
            <label>Mutation interval (ms):</label>
            <input
              type="number"
              value={intervalMs}
              onChange={(event) => {
                const value = Number(event.target.value);
                if (isNaN(value)) {
                  setError("Interval must be a number");
                } else {
                  setIntervalMs(value);
                }
              }}
            />
            <label>Active?:</label>
            <input
              type="checkbox"
              checked={!isPaused}
              onChange={(event) => setIsPaused(!event.target.checked)}
            />
          </div>
          <div style={{ display: "flex", gap: "10px", alignItems: "baseline" }}>
            <label>Show page boundaries:</label>
            <input
              type="checkbox"
              checked={showPageBoundaries}
              onChange={(event) => setShowPageBoundaries(event.target.checked)}
            />
          </div>
          <div style={{ display: "flex", gap: "10px", alignItems: "baseline" }}>
            <div>{`Sort ascending?`}</div>
            <input
              type="checkbox"
              checked={sortOrder === "asc"}
              onChange={(event) =>
                setSortOrder(event.target.checked ? "asc" : "desc")
              }
            />
          </div>
          <div style={{ display: "flex", gap: "10px", alignItems: "baseline" }}>
            <div>{`Query list twice?`}</div>
            <input
              type="checkbox"
              checked={double}
              onChange={(event) => setDouble(event.target.checked)}
            />
          </div>
          <div style={{ display: "flex", gap: "10px", alignItems: "baseline" }}>
            <button
              onClick={() => {
                setIsPaused(true);
                resetData()
                  .then(() => {
                    setResetCounter(resetCounter + 1);
                  })
                  .catch((e) => {
                    setError(`Error while resetting data: ${e.message}`);
                  });
              }}
            >
              Reset
            </button>
            <span>Reset counter: {resetCounter}</span>
          </div>

          <div style={{ display: "flex", gap: "10px" }}>
            <span>Last number: {lastNumber ?? "none"}</span>
          </div>
        </div>
      </div>
      <div
        style={{
          display: "flex",
          flexDirection: "row",
          gap: "10px",
        }}
      >
        <Inner
          key={resetCounter}
          sortOrder={sortOrder}
          showPageBoundaries={showPageBoundaries}
        />
        {double && (
          <Inner
            key={resetCounter + "x"}
            sortOrder={sortOrder}
            showPageBoundaries={showPageBoundaries}
          />
        )}
      </div>
    </main>
  );
}

function Inner({
  sortOrder,
  showPageBoundaries,
}: {
  sortOrder: "asc" | "desc";
  showPageBoundaries: boolean;
}) {
  const { results, status, loadMore } = usePaginatedQuery(
    api.numbers.listNumbers,
    { sortOrder },
    { initialNumItems: 5 },
  );
  useEffect(() => {
    let key =
      sortOrder === "asc"
        ? [-1, 0]
        : [Number.MAX_SAFE_INTEGER, Number.MAX_SAFE_INTEGER];
    for (const result of results) {
      const nextKey = [result.number, result._creationTime];
      const cmp = compareValues(key, nextKey);
      if (sortOrder === "asc" && cmp > 0) {
        console.error("out of order", results);
        throw new Error("out of order");
      } else if (sortOrder === "desc" && cmp < 0) {
        console.error("out of order", results);
        throw new Error("out of order");
      }
      key = nextKey;
    }
  }, [results, sortOrder]);

  const [rank, setRank] = useState<number>(100);
  const addNumber = useMutation(api.numbers.insert).withOptimisticUpdate(
    (localQueryStore, args) => {
      insertNumberOptimisticUpdate(sortOrder, localQueryStore, args);
    },
  );
  async function handleSendMessage(event: FormEvent) {
    event.preventDefault();
    await addNumber({
      number: rank,
      id: crypto.randomUUID(),
    });
  }
  return (
    <div
      style={{
        flexGrow: 1,
        display: "flex",
        flexDirection: "column",
        gap: "10px",
      }}
    >
      <form
        onSubmit={handleSendMessage}
        style={{ display: "flex", gap: "10px", alignItems: "baseline" }}
      >
        <label>Rank:</label>
        <input
          type="number"
          value={rank}
          onChange={(event) => setRank(Number(event.target.value))}
        />
        <input type="submit" value="Add" />
      </form>
      <ul>
        {results.map((number) => (
          <li key={number.id}>
            <span
              style={{
                color: number.color ?? "black",
                fontWeight: number.color === undefined ? "normal" : "bold",
              }}
            >
              {number.number}
            </span>
            {showPageBoundaries && number.isStart && <span>--- start</span>}
            {showPageBoundaries && number.isEnd && <span>--- end</span>}
          </li>
        ))}
      </ul>
      <div className="footer">
        <button
          onClick={() => {
            loadMore(5);
          }}
          disabled={status !== "CanLoadMore"}
        >
          Load More
        </button>
      </div>
    </div>
  );
}
