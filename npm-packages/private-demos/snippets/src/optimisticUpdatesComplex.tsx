import { api } from "../convex/_generated/api";
import { useMutation } from "convex/react";
import { Id } from "../convex/_generated/dataModel";

export function MessageSender(props: { channel: Id<"channels"> }) {
  const sendMessage = useMutation(api.messages.send).withOptimisticUpdate(
    (localStore, args) => {
      const { channel, body } = args;
      const existingMessages = localStore.getQuery(api.messages.list, {
        channel,
      });
      // If we've loaded the api.messages.list query, push an optimistic message
      // onto the list.
      if (existingMessages !== undefined) {
        const now = Date.now();
        const newMessage = {
          _id: crypto.randomUUID() as Id<"messages">,
          _creationTime: now,
          channel,
          body,
        };
        localStore.setQuery(api.messages.list, { channel }, [
          ...existingMessages,
          newMessage,
        ]);
      }
    },
  );

  async function handleSendMessage(
    channelId: Id<"channels">,
    newMessageText: string,
  ) {
    await sendMessage({ channel: channelId, body: newMessageText });
  }

  return (
    <button onClick={() => handleSendMessage(props.channel, "Hello world!")}>
      Send message
    </button>
  );
}
