import { FormEvent, useState } from "react";
import { useMutation, useQuery } from "convex/react";
import { api } from "../convex/_generated/api";
import { Status } from "../convex/messages";
import { convexToJson } from "convex/values";

export default function App() {
  const messages = useQuery(api.messages.list) || [];

  const [newMessageText, setNewMessageText] = useState("");
  const sendMessage = useMutation(api.messages.send);

  const [user] = useState(() => Math.floor(Math.random() * 10000).toString());
  async function handleSendMessage(event: FormEvent) {
    event.preventDefault();
    await sendMessage({ body: newMessageText, user });
    setNewMessageText("");
  }

  const status = useQuery(api.messages.getStatus, { user });

  return (
    <main>
      <h1>Convex Chat</h1>
      <p className="badge">
        <span>{`User ${user}`}</span>
      </p>
      {status && <Waitlist status={status} user={user} />}
      <div>{JSON.stringify(convexToJson((status ?? null) as any))}</div>
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
          disabled={!newMessageText || status?.status !== "InConversation"}
        />
      </form>
    </main>
  );
}

function Waitlist({ status, user }: { status: Status; user: string }) {
  const join = useMutation(api.waitlist.join);
  const leave = useMutation(api.waitlist.leave);
  switch (status.status) {
    case "RemovedFromConversation":
      return (
        <div>
          <div>Time's up!</div>
          <button onClick={() => join({ user })}>Join waitlist</button>
        </div>
      );
    case "InConversation":
      return <div>You're in! Start talking!</div>;
    case "NotOnWaitlist":
      return <button onClick={() => join({ user })}>Join waitlist</button>;
    case "OnWaitlist": {
      const position = Number(status.position - status.headPosition);
      const numPositions = Number(status.tailPosition - status.headPosition);
      const progressPct =
        numPositions === 0
          ? 100
          : 5 + Math.round(((numPositions - position) / numPositions) * 95);
      return (
        <div>
          <div>You're on the waitlist!</div>
          <div>{progressPct}%</div>
          <button onClick={() => leave({ user })}>Leave waitlist</button>
        </div>
      );
    }
  }
}
