import { useState, FormEvent } from "react";
import { Id } from "../convex/_generated/dataModel";
import { useMutation, useQuery } from "convex/react";
import { api } from "../convex/_generated/api";

const randomName = "User " + Math.floor(Math.random() * 10000);

export default function App() {
  // Dynamically update `messages` in response to the output of
  // `listMessages.ts`.
  const messages =
    useQuery(api.listMessages.default, { channel: "#general" }) || [];
  const sendMessage = useMutation(api.sendMessage.default).withOptimisticUpdate(
    (localStore, { channel, body, author }) => {
      const existingMessages = localStore.getQuery(api.listMessages.default, {
        channel,
      });
      // If we've loaded the listMessages query, push an optimistic message
      // onto the list.
      if (existingMessages !== undefined) {
        const now = Date.now();
        const newMessage = {
          _id: crypto.randomUUID() as Id<"messages">,
          _creationTime: now,
          channel,
          body,
          author,
        };
        localStore.setQuery(api.listMessages.default, { channel }, [
          ...existingMessages,
          newMessage,
        ]);
      }
    },
  );

  // Run `sendMessage.ts` as a transaction to record a chat message when
  // `handleSubmit` triggered.
  const [text, setText] = useState("");
  async function handleSubmit(event: FormEvent) {
    event.preventDefault();
    if (text) {
      setText(""); // reset text entry box
      await sendMessage({
        channel: "#general",
        body: text,
        author: randomName,
      });
    }
  }

  return (
    <main className="py-4">
      <h1 className="text-center">Convex Chat</h1>
      <p className="text-center">
        <span className="badge bg-dark">{randomName}</span>
      </p>
      <ul className="list-group shadow-sm my-3">
        {messages.slice(-10).map((message) => (
          <li
            key={message._id}
            className="list-group-item d-flex justify-content-between"
          >
            <div>
              <strong>{message.author}:</strong> {message.body}
            </div>
            <div className="ml-auto text-secondary text-nowrap">
              {new Date(message._creationTime).toLocaleTimeString()}
            </div>
          </li>
        ))}
      </ul>
      <form onSubmit={handleSubmit} className="d-flex justify-content-center">
        <input
          value={text}
          onChange={(event) => setText(event.target.value)}
          className="form-control w-50"
          placeholder="Write a message…"
        />
        <input type="submit" value="Send" className="ms-2 btn btn-primary" />
      </form>
    </main>
  );
}
