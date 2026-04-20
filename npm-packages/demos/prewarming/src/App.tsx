import { FormEvent, useRef, useState } from "react";
import { useConvex, useMutation, useQuery } from "convex/react";
import { api } from "../convex/_generated/api";

const query = api.messages.list;
const args = {};

export default function App() {
  const [activePane, setActivePane] = useState<"chat" | "about">("about");
  const [prewarmText, setPrewarmText] = useState("");
  const clearMessage = useRef<ReturnType<typeof setTimeout>>();
  const convex = useConvex();
  return (
    <main>
      <nav>
        <button
          onClick={() => setActivePane("about")}
          disabled={activePane === "about"}
        >
          about
        </button>
        <button
          onClick={() => setActivePane("chat")}
          disabled={activePane === "chat"}
        >
          chat
        </button>
        <button
          onClick={() => setActivePane("chat")}
          onMouseEnter={() => {
            convex.prewarmQuery({ query, args });
            setPrewarmText(" (prewarming now!)");
            clearTimeout(clearMessage.current);
            clearMessage.current = setTimeout(() => setPrewarmText(""), 5000);
          }}
          disabled={activePane === "chat"}
        >
          chat with prewarm on hover{prewarmText}
        </button>
      </nav>
      {activePane === "chat" ? <Chat /> : <About />}
    </main>
  );
}

function About() {
  return (
    <div>
      <h1>Convex Chat</h1> In the other pane is a chat app.
    </div>
  );
}

function Chat() {
  const messages = useQuery(query) || [];

  const [newMessageText, setNewMessageText] = useState("");
  const sendMessage = useMutation(api.messages.send);

  const [name] = useState(() => "User " + Math.floor(Math.random() * 10000));
  async function handleSendMessage(event: FormEvent) {
    event.preventDefault();
    await sendMessage({ body: newMessageText, author: name });
    setNewMessageText("");
  }
  return (
    <div>
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
    </div>
  );
}
