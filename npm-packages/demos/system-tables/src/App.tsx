import { FormEvent, useRef, useState } from "react";
import { useMutation, useQuery } from "convex/react";
import { api } from "../convex/_generated/api";

export default function App() {
  const messages = useQuery(api.messages.list) || [];

  const [newMessageText, setNewMessageText] = useState("");
  const sendMessage = useMutation(api.messages.sendMessage);
  const scheduleMessage = useMutation(api.messages.scheduleMessage);

  const [name] = useState(() => "User " + Math.floor(Math.random() * 10000));

  async function handleSendMessage(event: FormEvent) {
    event.preventDefault();
    if (newMessageText.startsWith("/delay ")) {
      const args = newMessageText.split(" ");
      if (args.length < 3) {
        throw new Error("Invalid arguments");
      }
      const delay = args[1];
      const delaySeconds = parseInt(delay);

      if (isNaN(delaySeconds)) {
        throw new Error("Invalid delay");
      }
      await scheduleMessage({
        body: args.slice(2).join(" "),
        author: name,
        delay: delaySeconds,
      });
    } else {
      await sendMessage({ body: newMessageText, author: name });
    }
    setNewMessageText("");
  }

  const generateUploadUrl = useMutation(api.messages.generateUploadUrl);
  const sendImage = useMutation(api.messages.sendImage);

  const imageInput = useRef<HTMLInputElement>(null);
  const [selectedImage, setSelectedImage] = useState<File | null>(null);

  async function handleSendImage(event: FormEvent) {
    event.preventDefault();

    // Step 1: Get a short-lived upload URL
    const postUrl = await generateUploadUrl();
    // Step 2: POST the file to the URL
    const result = await fetch(postUrl, {
      method: "POST",
      headers: { "Content-Type": selectedImage!.type },
      body: selectedImage,
    });
    const json = await result.json();
    if (!result.ok) {
      throw new Error(`Upload failed: ${JSON.stringify(json)}`);
    }
    const { storageId } = json;
    // Step 3: Save the newly allocated storage id to the database
    await sendImage({ storageId, author: name });

    setSelectedImage(null);
    imageInput.current!.value = "";
  }

  return (
    <main>
      <h1>Convex Chat</h1>
      <p className="badge">
        <span>{name}</span>
      </p>
      <div className="instructions">
        To schedule send a message, use{" "}
        <span>/delay (# of seconds) message</span>
      </div>
      <ul>
        {messages.map((message) => (
          <li key={message._id}>
            <span>{message.author}:</span>
            {message.format === "image" ? (
              <Image message={message} />
            ) : (
              <span>{message.body}</span>
            )}
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
      <form onSubmit={handleSendImage}>
        <input
          type="file"
          accept="image/*"
          ref={imageInput}
          onChange={(event) => setSelectedImage(event.target.files![0])}
          className="ms-2 btn btn-primary"
          disabled={selectedImage !== null}
        />
        <input
          type="submit"
          value="Send Image"
          disabled={selectedImage === null}
        />
      </form>
    </main>
  );
}

function Image({ message }: { message: { url?: string | null | undefined } }) {
  if (message.url !== undefined && message.url !== null) {
    return <img src={message.url} height="300px" width="auto" />;
  }
  return <></>;
}
