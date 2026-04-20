import { FormEvent, useState } from "react";
import { useAction, useMutation, useQuery } from "convex/react";
import { api } from "../convex/_generated/api";
import { SearchResult } from "../convex/movies";
import { GENRES } from "../convex/constants";
import { Id } from "../convex/_generated/dataModel";

function Insert() {
  const [title, setTitle] = useState("");
  const [description, setDescription] = useState("");
  const [genre, setGenre] = useState("Action");
  const [insertInProgress, setInsertInProgress] = useState(false);
  const insert = useMutation(api.movies.insert);

  async function handleInsert(event: FormEvent) {
    event.preventDefault();
    setInsertInProgress(true);
    try {
      await insert({ description, genre, title });
      setDescription("");
    } finally {
      setInsertInProgress(false);
    }
  }
  return (
    <>
      <h2>Add a new movie</h2>
      <form onSubmit={handleInsert}>
        <textarea
          value={title}
          onChange={(event) => setTitle(event.target.value)}
          placeholder="Title"
        />
        <textarea
          value={description}
          onChange={(event) => setDescription(event.target.value)}
          placeholder="Description"
        />
        <select value={genre} onChange={(e) => setGenre(e.target.value)}>
          {Object.entries(GENRES).map(([c, e]) => (
            <option key={c} value={c}>
              {presentGenre(c, e)}
            </option>
          ))}
        </select>
        <input
          type="submit"
          value="Insert"
          disabled={!description || !title || insertInProgress}
        />
      </form>
    </>
  );
}

function presentGenre(name: string, emoji: string) {
  return `${emoji} ${name[0].toUpperCase()}${name.slice(1)}`;
}

function Search() {
  const [searchText, setSearchText] = useState("");
  const [searchFilter, setSearchFilter] = useState<string[]>([]);
  const [searchResults, setSearchResults] = useState<
    SearchResult[] | undefined
  >();
  const [searchInProgress, setSearchInProgress] = useState(false);

  const vectorSearch = useAction(api.search.similarMovies);

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
        genres: searchFilter.length > 0 ? searchFilter : undefined,
      });
      setSearchResults(results);
    } finally {
      setSearchInProgress(false);
    }
  };
  return (
    <>
      <h2>Search movies (Cmd-click to add filters)</h2>
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
          {Object.entries(GENRES).map(([c, e]) => (
            <option key={c} value={c}>
              {presentGenre(c, e)}
            </option>
          ))}
        </select>
        <input type="submit" value="Search" disabled={searchInProgress} />
      </form>
      <div>
        <h3>Vector Results</h3>
        {searchResults !== undefined && (
          <SearchResults searchResults={searchResults} />
        )}
      </div>
    </>
  );
}

function SearchResults(props: { searchResults: SearchResult[] }) {
  const searchResults = useQuery(api.movies.fetchResults, {
    results: props.searchResults,
  });
  return searchResults === undefined ? (
    <div>Loading..</div>
  ) : (
    <ul>
      {searchResults.map((result) => (
        <li key={result._id}>
          <span>{(GENRES as any)[result.genre]}</span>
          <span>{result.title}</span>
          <span>{result._score.toFixed(4)}</span>
          <Vote movieId={result._id} />
          <span>{`Votes: ${result.votes}`}</span>
        </li>
      ))}
    </ul>
  );
}

function Vote(props: { movieId: Id<"movies"> }) {
  const upvote = useMutation(api.movies.upvote);
  const downvote = useMutation(api.movies.downvote);
  return (
    <div>
      <button onClick={() => void upvote({ id: props.movieId })}>👍</button>
      <button onClick={() => void downvote({ id: props.movieId })}>👎</button>
    </div>
  );
}

function Populate() {
  const populate = useAction(api.movies.populate);
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

export default function Movies() {
  const entries = useQuery(api.movies.list);

  return (
    <main>
      <h1>🎬 Movie vector search</h1>
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
              <span>{(GENRES as any)[entry.genre]}</span>
              <span>{entry.title}</span>
              <span>{`Votes: ${entry.votes}`}</span>
              <Vote movieId={entry._id} />
            </li>
          ))}
        </ul>
      )}
      <Insert />
      <Search />
    </main>
  );
}
