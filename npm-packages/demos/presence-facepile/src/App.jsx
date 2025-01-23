import { useState } from "react";
import { useMutation, useQuery } from "convex/react";
import { api } from "../convex/_generated/api";
import FacePile from "./Facepile";
import usePresence from "./hooks/usePresence";

const Emojis =
  "😀 😃 😄 😁 😆 😅 😂 🤣 🥲 🥹 😊 😇 🙂 🙃 😉 😌 😍 🥰 😘 😗 😙 😚 😋 😛 😝 😜 🤪 😎 🥸 🤩 🥳 😏 😳 🤔 🫢 🤭 🤫 😶 🫠 😮 🤤 😵‍💫 🥴 🤑 🤠".split(
    " ",
  );

const initialEmoji = Emojis[Math.floor(Math.random() * Emojis.length)];

export default function App() {
  const messages = useQuery(api.listMessages.default) || [];

  const [newMessageText, setNewMessageText] = useState("");
  const sendMessage = useMutation(api.sendMessage.default);

  const [name] = useState(() => "User " + Math.floor(Math.random() * 10000));
  const [myPresence, othersPresence, updateMyPresence] = usePresence(
    "chat-room",
    name,
    {
      name,
      emoji: initialEmoji,
    },
  );
  async function handleSendMessage(event) {
    event.preventDefault();
    await sendMessage({ body: newMessageText, author: name });
    setNewMessageText("");
  }
  return (
    <main>
      <h1>Convex Chat</h1>
      <p className="badge">
        <span>{name}</span>
        <select
          defaultValue={myPresence.emoji}
          onChange={(e) => updateMyPresence({ emoji: e.target.value })}
        >
          {Emojis.map((e) => (
            <option key={e}>{e}</option>
          ))}
        </select>
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
        <FacePile othersPresence={othersPresence ?? []} />
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
