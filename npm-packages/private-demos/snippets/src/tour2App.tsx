import { useMutation, useQuery } from "convex/react";
import { api } from "../convex/_generated/api";

const NAME = "Foo";

export function App() {
  const likeMessage = useMutation(api.messages.like);
  const message = useQuery(api.messages.list, { channel: "foo" })?.[0];
  return (
    // @snippet start button
    <p>
      {message.body}
      <button
        onClick={async () => {
          await likeMessage({ liker: NAME, messageId: message._id });
        }}
      >
        🤍
      </button>
    </p>
    // @snippet end button
  );
}
