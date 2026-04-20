import { FormEvent, useState } from "react";
import { useMutation, usePaginatedQuery } from "convex/react";
import { api } from "../convex/_generated/api";

export default function App() {
  const { results, status, loadMore } = usePaginatedQuery(
    api.messages.list,
    {},
    { initialNumItems: 5 },
  );

  const [newMessageText, setNewMessageText] = useState("");
  const sendMessage = useMutation(api.messages.send);

  const [name] = useState(() => "User " + Math.floor(Math.random() * 10000));
  async function handleSendMessage(event: FormEvent) {
    event.preventDefault();
    await sendMessage({ body: newMessageText, author: name });
    setNewMessageText("");
  }
  return (
    <main>
      <h1>Convex Chat</h1>
      <p className="badge">
        <span>{name}</span>
      </p>
      <form onSubmit={handleSendMessage}>
        <input
          value={newMessageText}
          onChange={(event) => setNewMessageText(event.target.value)}
          placeholder="Write a message…"
        />
        <input type="submit" value="Send" disabled={!newMessageText} />
      </form>
      <ul>
        {results.map((message) => (
          <li key={message._id}>
            <span>{message.author}:</span>
            <span>{message.body}</span>
            <span>{new Date(message._creationTime).toLocaleTimeString()}</span>
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
    </main>
  );
}
