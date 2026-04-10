import { FormEvent, useState } from "react";
import { useMutation, useQuery } from "convex/react";
import { api } from "../convex/_generated/api";

export default function App() {
  const messages =
    useQuery({ query: api.messages.list, args: {}, throwOnError: true }).data ??
    [];

  const [newMessageText, setNewMessageText] = useState("");
  const sendMessage = useMutation(api.messages.send);

  const [name] = useState(() => "User " + Math.floor(Math.random() * 10000));
  async function handleSendMessage(event: FormEvent) {
    event.preventDefault();
    await sendMessage({ body: newMessageText, author: name });
    setNewMessageText("");
  }

  const [searchText, setSearchText] = useState("");
  const searchResults =
    useQuery({ query: api.messages.search, args: { query: searchText } })
      ?.data ?? [];

  return (
    <main>
      <h1>Convex Chat</h1>
      <p className="badge">
        <span>{name}</span>
      </p>
      <ul>
        {messages.map((message) => (
          <li key={message._id}>
            <span>{message.author}:</span>
            <span>{message.body}</span>
            <span>{new Date(message._creationTime).toLocaleTimeString()}</span>
          </li>
        ))}
      </ul>
      <form onSubmit={handleSendMessage}>
        <input
          value={newMessageText}
          onChange={(event) => setNewMessageText(event.target.value)}
          placeholder="Write a message…"
        />
        <input type="submit" value="Send" disabled={!newMessageText} />
      </form>
      <div className="search">
        <h2>Search Messages</h2>
        <input
          value={searchText}
          onChange={(event) => setSearchText(event.target.value)}
          placeholder="Search!"
        />
        <ul>
          {searchResults.map((searchResult) => (
            <li key={searchResult._id}>
              <span>{searchResult.author}:</span>
              <span>{searchResult.body}</span>
              <span>
                {new Date(searchResult._creationTime).toLocaleTimeString()}
              </span>
            </li>
          ))}
        </ul>
      </div>
    </main>
  );
}
