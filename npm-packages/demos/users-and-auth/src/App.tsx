import { FormEvent, useState } from "react";
import { useMutation, useQuery } from "convex/react";
import { api } from "../convex/_generated/api";
import Badge from "./Badge";
import LogoutButton from "./LogoutButton";
import useStoreUserEffect from "./useStoreUserEffect";

export default function App() {
  const userId = useStoreUserEffect();

  const messages = useQuery(api.messages.list) || [];

  const [newMessageText, setNewMessageText] = useState("");
  const sendMessage = useMutation(api.messages.send);

  async function handleSendMessage(event: FormEvent) {
    event.preventDefault();
    await sendMessage({ body: newMessageText });
    setNewMessageText("");
  }
  return (
    <main>
      <h1>Convex Chat</h1>
      {Badge()}
      <h2>{LogoutButton()}</h2>
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
        <input
          type="submit"
          value="Send"
          disabled={newMessageText === "" || userId === null}
        />
      </form>
    </main>
  );
}
