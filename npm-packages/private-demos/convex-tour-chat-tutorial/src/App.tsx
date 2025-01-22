import { useQuery, useMutation } from "convex/react";
import { api } from "../convex/_generated/api";
import { useEffect, useState } from "react";
import { faker } from "@faker-js/faker";

// For demo purposes. In a real app, you'd have real user data.
const NAME = faker.person.firstName();

export default function App() {
  const messages = useQuery(api.messages.list);
  const sendMessage = useMutation(api.messages.send);

  const [newMessageText, setNewMessageText] = useState("");

  useEffect(() => {
    // Make sure scrollTo works on button click in Chrome
    setTimeout(() => {
      window.scrollTo({ top: document.body.scrollHeight, behavior: "smooth" });
    }, 0);
  }, [messages]);

  return (
    <main className="chat">
      <header>
        <h1>Convex Chat</h1>
        <p>
          Connected as <strong>{NAME}</strong>
        </p>
      </header>
      {messages?.map((message) => (
        <article
          key={message._id}
          className={message.author === NAME ? "message-mine" : ""}
        >
          <div>{message.author}</div>

          <p>{message.body}</p>
        </article>
      ))}
      <form
        onSubmit={async (e) => {
          e.preventDefault();
          await sendMessage({ body: newMessageText, author: NAME });
          setNewMessageText("");
        }}
      >
        <input
          value={newMessageText}
          onChange={async (e) => {
            const text = e.target.value;
            setNewMessageText(text);
          }}
          placeholder="Write a message…"
        />
        <button type="submit" disabled={!newMessageText}>
          Send
        </button>
      </form>
    </main>
  );
}
