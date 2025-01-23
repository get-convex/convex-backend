import { useState } from "react";
// @snippet start listMessages
// @snippet start importHooks
import { useMutation, useQuery } from "convex/react";
// @snippet end importHooks

export default function App() {
  const messages = useQuery("messages:list") || [];

  const [newMessageText, setNewMessageText] = useState("");
  // @snippet start sendMessage
  // @snippet start sendMessageHook
  const sendMessage = useMutation("messages:send");
  // @snippet end sendMessageHook

  const [name] = useState(() => "User " + Math.floor(Math.random() * 10000));
  async function handleSendMessage(event) {
    event.preventDefault();
    await sendMessage({ body: newMessageText, author: name });
    setNewMessageText("");
  }
  // @snippet end sendMessage
  // @snippet end listMessages
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
