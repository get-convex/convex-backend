import { FormEvent, useState } from "react";
import { useQuery } from "convex/react";
import {
  useSessionMutation,
  useSessionQuery,
} from "convex-helpers/react/sessions";
import { api } from "../convex/_generated/api";

export default function App() {
  const messages = useQuery(api.messages.list) || [];

  const [newMessageText, setNewMessageText] = useState("");
  const sendMessage = useSessionMutation(api.messages.send);

  async function handleSendMessage(event: FormEvent) {
    event.preventDefault();
    await sendMessage({ body: newMessageText });
    setNewMessageText("");
  }
  return (
    <main>
      <h1>Convex Chat</h1>
      <p className="badge">
        <Name />
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
    </main>
  );
}

function Name() {
  const name = useSessionQuery(api.name.get);
  const updateName = useSessionMutation(api.name.set);
  return (
    <span>
      <input
        name="name"
        value={name ?? ""}
        type="text"
        onChange={(e) => updateName({ name: e.target.value })}
        placeholder="Type Name"
      />
    </span>
  );
}
