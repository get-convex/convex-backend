import { FormEvent, useState } from "react";
import { api } from "../convex/_generated/api";
import { useMutation, useQuery } from "convex/react";
import ChatBox from "./ChatBox";
import { Id } from "../convex/_generated/dataModel";

export default function App() {
  const channels = useQuery(api.channels.list) || [];
  const [channelId, setChannelId] = useState<Id<"channels"> | null>(null);
  const [newChannelName, setNewChannelName] = useState("");
  const addChannel = useMutation(api.channels.add);
  const [name] = useState(() => "User " + Math.floor(Math.random() * 10000));

  async function handleAddChannel(event: FormEvent) {
    event.preventDefault();
    const id = await addChannel({ name: newChannelName });
    setNewChannelName("");
    setChannelId(id);
  }
  return (
    <main>
      <h1>Convex Chat</h1>
      <p className="badge">
        <span>{name}</span>
      </p>
      <div className="main-content">
        <div className="channel-box">
          <ul>
            {channels.map((channel) => (
              <li
                key={channel._id}
                onClick={() => setChannelId(channel._id)}
                style={{
                  fontWeight: channel._id === channelId ? "bold" : "normal",
                }}
              >
                {channel.name}
              </li>
            ))}
          </ul>
          <form onSubmit={handleAddChannel}>
            <input
              value={newChannelName}
              onChange={(event) => setNewChannelName(event.target.value)}
              placeholder="Add a channel..."
            />
            <input type="submit" value="Add" disabled={!newChannelName} />
          </form>
        </div>
        {channelId ? <ChatBox channelId={channelId} name={name} /> : null}
      </div>
    </main>
  );
}
