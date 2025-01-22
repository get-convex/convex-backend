import { FormEvent, useState } from "react";
import { useMutation, useQuery } from "convex/react";
import { api } from "../convex/_generated/api";
import { ConvexError } from "convex/values";
import ErrorBoundary from "./ErrorBoundary";

export default function App() {
  const [newMessageText, setNewMessageText] = useState("");
  const sendMessage = useMutation(api.messages.sendMessage);

  const [name] = useState(() => "User " + Math.floor(Math.random() * 10000));
  async function handleSendMessage(event: FormEvent) {
    event.preventDefault();
    try {
      if (newMessageText) {
        await sendMessage({ body: newMessageText, author: name });
      }
      setNewMessageText("");
    } catch (error) {
      const errorMessage =
        error instanceof ConvexError ? error.data : "Unexpected error occurred";
      alert(errorMessage);
    }
  }
  const clearMessages = useMutation(api.messages.clearMessages);

  return (
    <main>
      <h1>Convex Chat</h1>
      <p className="badge">
        <span>{name}</span>
      </p>
      <ErrorBoundary clearMessages={clearMessages}>
        <MessageList />
        <form onSubmit={handleSendMessage}>
          <input
            value={newMessageText}
            onChange={(event) => setNewMessageText(event.target.value)}
            placeholder="Write a message…"
          />
          <input type="submit" value="Send" disabled={!newMessageText} />
        </form>
      </ErrorBoundary>
    </main>
  );
}

export function MessageList() {
  const messages = useQuery(api.messages.list) || [];
  return (
    <ul>
      {messages.map((message) => (
        <li key={message._id}>
          <span>{message.author}:</span>
          <span>{message.body}</span>
          <span>{new Date(message._creationTime).toLocaleTimeString()}</span>
        </li>
      ))}
    </ul>
  );
}
