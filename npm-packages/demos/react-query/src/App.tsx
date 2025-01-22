import { FormEvent, useState } from "react";
import { useMutation, useQuery } from "@tanstack/react-query";
import { convexQuery, useConvexMutation } from "@convex-dev/react-query";
import { api } from "../convex/_generated/api";

export default function App() {
  // @snippet start useQuery
  const { data, error, isPending } = useQuery(
    convexQuery(api.messages.list, {}),
  );
  // @snippet end useQuery

  const [newMessageText, setNewMessageText] = useState("");
  // @snippet start useMutation
  const { mutate, isPending: sending } = useMutation({
    mutationFn: useConvexMutation(api.messages.send),
  });
  // @snippet end useMutation

  const [name] = useState(() => "User " + Math.floor(Math.random() * 10000));
  async function handleSendMessage(event: FormEvent) {
    event.preventDefault();
    if (!sending && newMessageText) {
      mutate(
        { body: newMessageText, author: name },
        {
          onSuccess: () => setNewMessageText(""),
        },
      );
    }
  }
  if (error) {
    return <div>Error: {error.toString()}</div>;
  }
  if (isPending) {
    return <div>loading...</div>;
  }
  return (
    <main>
      <h1>Convex Chat</h1>
      <p className="badge">
        <span>{name}</span>
      </p>
      <ul>
        {data.map((message) => (
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
        <input type="submit" value="Send" />
      </form>
    </main>
  );
}
