import { useEffect, useState } from "react";
import { useMutation, useQuery } from "convex/react";
import { api } from "../convex/_generated/api";
import usePresence, { isOnline } from "./hooks/usePresence";

export default function App() {
  const messages = useQuery(api.messages.list) || [];

  const [newMessageText, setNewMessageText] = useState("");
  const sendMessage = useMutation(api.messages.send);

  const [name] = useState(() => "User " + Math.floor(Math.random() * 10000));
  const [, othersPresence, updateMyPresence] = usePresence("chat-room", name, {
    typing: false,
  });

  async function handleSendMessage(event) {
    event.preventDefault();
    await sendMessage({ body: newMessageText, author: name });
    setNewMessageText("");
  }

  useEffect(() => {
    if (newMessageText.length === 0) {
      updateMyPresence({ typing: false });
      return;
    }
    updateMyPresence({ typing: true });
    const timer = setTimeout(() => updateMyPresence({ typing: false }), 1000);
    return () => clearTimeout(timer);
  }, [updateMyPresence, newMessageText]);

  return (
    <main>
      <h1>Convex Chat</h1>
      <p className="badge">
        <span>{name}</span>
      </p>
      {/* @snippet start renderMessages */}
      <ul>
        {messages.map((message) => (
          <li key={message._id}>
            <span>{message.author}:</span>
            <span>{message.body}</span>
            <span>{new Date(message._creationTime).toLocaleTimeString()}</span>
          </li>
        ))}
        {othersPresence
          ?.filter(isOnline)
          .filter(({ data }) => data.typing)
          .map(({ user }) => (
            <li key={user}>
              <span>{user}</span>
              <span>
                <i>typing...</i>
              </span>
            </li>
          ))}
      </ul>
      {/* @snippet end renderMessages */}
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
