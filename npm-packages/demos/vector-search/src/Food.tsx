import { FormEvent, useState } from "react";
import { useAction, useQuery } from "convex/react";
import { api } from "../convex/_generated/api";
import { SearchResult } from "../convex/foods";
import { CUISINES } from "../convex/constants";

function Insert() {
  const [description, setDescription] = useState("");
  const [cuisine, setCuisine] = useState("american");
  const [insertInProgress, setInsertInProgress] = useState(false);
  const insert = useAction(api.foods.insert);

  async function handleInsert(event: FormEvent) {
    event.preventDefault();
    setInsertInProgress(true);
    try {
      await insert({ description, cuisine });
      setDescription("");
    } finally {
      setInsertInProgress(false);
    }
  }
  return (
    <>
      <h2>Add a new food</h2>
      <form onSubmit={handleInsert}>
        <textarea
          value={description}
          onChange={(event) => setDescription(event.target.value)}
          placeholder="Description"
        />
        <select value={cuisine} onChange={(e) => setCuisine(e.target.value)}>
          {Object.entries(CUISINES).map(([c, e]) => (
            <option key={c} value={c}>
              {presentCuisine(c, e)}
            </option>
          ))}
        </select>
        <input
          type="submit"
          value="Insert"
          disabled={!description || insertInProgress}
        />
      </form>
    </>
  );
}

function presentCuisine(name: string, emoji: string) {
  return `${emoji} ${name[0].toUpperCase()}${name.slice(1)}`;
}

function Search() {
  const [searchText, setSearchText] = useState("");
  const [searchFilter, setSearchFilter] = useState<string[]>([]);
  const [searchResults, setSearchResults] = useState<
    SearchResult[] | undefined
  >();
  const [searchInProgress, setSearchInProgress] = useState(false);

  const vectorSearch = useAction(api.search.similarFoods);

  const handleSearch = async (event: FormEvent) => {
    event.preventDefault();
    setSearchResults(undefined);
    if (!searchText) {
      return;
    }
    setSearchInProgress(true);
    try {
      const results = await vectorSearch({
        query: searchText,
        cuisines: searchFilter.length > 0 ? searchFilter : undefined,
      });
      setSearchResults(results);
    } finally {
      setSearchInProgress(false);
    }
  };
  return (
    <>
      <h2>Search foods (Cmd-click to add filters)</h2>
      <form onSubmit={handleSearch}>
        <input
          value={searchText}
          onChange={(e) => setSearchText(e.target.value)}
          placeholder="Query"
        />
        <select
          value={searchFilter}
          multiple={true}
          onChange={(e) =>
            setSearchFilter([...e.target.selectedOptions].map((o) => o.value))
          }
        >
          {Object.entries(CUISINES).map(([c, e]) => (
            <option key={c} value={c}>
              {presentCuisine(c, e)}
            </option>
          ))}
        </select>
        <input type="submit" value="Search" disabled={searchInProgress} />
      </form>
      <div>
        <h3>Vector Results</h3>
        {searchResults !== undefined && (
          <ul>
            {searchResults.map((result) => (
              <li key={result._id}>
                <span>{(CUISINES as any)[result.cuisine]}</span>
                <span>{result.description}</span>
                <span>{result._score.toFixed(4)}</span>
              </li>
            ))}
          </ul>
        )}
      </div>
    </>
  );
}

function Populate() {
  const populate = useAction(api.foods.populate);
  const [submitted, setSubmitted] = useState(false);
  return (
    <center>
      <i>No entries yet</i>
      <input
        type="button"
        value="Populate test data"
        onClick={() => {
          setSubmitted(true);
          populate();
        }}
        disabled={submitted}
      />
    </center>
  );
}

export default function Food() {
  const entries = useQuery(api.foods.list);

  return (
    <main>
      <h1>🍔 Food vector search</h1>
      <h2>Entries (ten most recent)</h2>
      {entries === undefined && (
        <center>
          <i>Loading...</i>
        </center>
      )}
      {entries !== undefined && entries.length === 0 && <Populate />}
      {entries && entries.length > 0 && (
        <ul>
          {entries.map((entry) => (
            <li key={entry._id}>
              <span>{(CUISINES as any)[entry.cuisine]}</span>
              <span>{entry.description}</span>
            </li>
          ))}
        </ul>
      )}
      <Insert />
      <Search />
    </main>
  );
}
